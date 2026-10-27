//! CPAL backend implementation
//!
//! This backend uses the CPAL library for cross-platform audio I/O.
//! On Linux and BSD, the native PipeWire host is preferred when available.

use anyhow::Result;
use audio_core::{AudioBackend, AudioCallback, AudioStream, DeviceId, DeviceInfo, StreamParams};
use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    BufferSize, Host, SampleFormat, Stream, StreamConfig, SupportedBufferSize,
    SupportedStreamConfig, SupportedStreamConfigRange,
};
use std::collections::HashSet;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;

fn id_ends_with_segment(id: &str, segment: &str) -> bool {
    id == segment || id.ends_with(&format!(":{segment}")) || id.ends_with(&format!("/{segment}"))
}

fn create_host() -> Result<Host> {
    #[cfg(all(
        feature = "pipewire",
        any(
            target_os = "linux",
            target_os = "dragonfly",
            target_os = "freebsd",
            target_os = "netbsd"
        )
    ))]
    {
        match cpal::host_from_id(cpal::HostId::PipeWire) {
            Ok(host) => {
                tracing::info!("Using CPAL PipeWire host");
                return Ok(host);
            }
            Err(e) => {
                tracing::warn!(
                    "PipeWire host unavailable ({e}), falling back to default CPAL host"
                );
            }
        }
    }

    Ok(cpal::default_host())
}

/// CPAL backend implementation
pub struct CpalBackend {
    host: Host,
}

/// Stream format chosen for a device: the supported range plus the rate to open it at.
///
/// The rate can differ from the configured one when the device cannot run it (WASAPI
/// shared mode is locked to the engine mix format); `engine-core` then runs the DSP at
/// the device rate.
#[derive(Clone, Copy, Debug)]
struct DeviceFormat {
    range: SupportedStreamConfigRange,
    sample_rate: u32,
}

impl CpalBackend {
    /// Create a new CPAL backend
    pub fn new() -> Result<Self> {
        let host = create_host()?;
        Ok(Self { host })
    }

    /// Build our DeviceId from CPAL device (uses stable id()).
    fn device_id(device: &cpal::Device) -> Result<DeviceId> {
        let id = device.id().map_err(anyhow::Error::msg)?;
        Ok(DeviceId::new(format!("cpal:{}", id)))
    }

    /// Resolve device display name using description() (comprehensive device info).
    fn device_name(device: &cpal::Device) -> String {
        device
            .description()
            .ok()
            .map(|d| d.name().to_string())
            .unwrap_or_else(|| "Unknown Device".to_string())
    }

    /// PipeWire virtual default sink — redundant with `output_default`.
    fn is_pipewire_sink_default(id: &str, name: &str) -> bool {
        name.eq_ignore_ascii_case("sink_default") || id_ends_with_segment(id, "sink_default")
    }

    /// PipeWire node for this (or another) client's playback stream, not a real sink.
    fn is_unnamed_stream_output(id: &str, name: &str) -> bool {
        name.eq_ignore_ascii_case("unknown")
            || name.eq_ignore_ascii_case("Unknown Device")
            || id_ends_with_segment(id, "mixar")
    }

    /// Whether this device should appear in `list_output_devices`.
    fn should_list_output_device(id: &str, name: &str) -> bool {
        !Self::is_pipewire_sink_default(id, name) && !Self::is_unnamed_stream_output(id, name)
    }

    /// Find device by stable id (from DeviceId).
    fn find_device_by_id(&self, device_id: &DeviceId) -> Result<cpal::Device> {
        let id_str = device_id.as_str();
        let suffix = id_str
            .strip_prefix("cpal:")
            .ok_or_else(|| anyhow::anyhow!("Invalid CPAL device ID format: {}", id_str))?;
        let cpal_id: cpal::DeviceId = suffix
            .parse()
            .map_err(|e| anyhow::anyhow!("Invalid CPAL device ID: {} ({e})", suffix))?;
        self.host
            .device_by_id(&cpal_id)
            .ok_or_else(|| anyhow::anyhow!("Device not found: {}", id_str))
    }

    /// Frames to request from a device for a configured buffer size.
    ///
    /// The configured size is the DSP chunk length, not something the device is obliged to
    /// produce per callback. WASAPI shared mode owns the callback period (the audio engine's
    /// ~10 ms, i.e. 480 frames at 48 kHz) and advertises it as `Range { min == max }`, so the
    /// only size such a device accepts is its period. Asking for the nearest supported size
    /// keeps the device openable; `engine-core` decouples the callback period from the DSP
    /// chunk through the ring buffer, so both may differ.
    fn negotiate_frames(supported: &SupportedBufferSize, requested: u32) -> u32 {
        match supported {
            SupportedBufferSize::Range { min, max } if min <= max => requested.clamp(*min, *max),
            _ => requested,
        }
    }

    /// Log the frames actually negotiated, so a period that differs from the configured
    /// chunk is visible instead of looking like a misconfiguration.
    fn log_device_buffer_limits_frames(supported: &SupportedBufferSize, requested: u32) {
        match supported {
            SupportedBufferSize::Range { min, max } => {
                let frames = Self::negotiate_frames(supported, requested);
                tracing::info!(
                    "Device supported buffer size range: {}..={} frames (requesting {} for a configured {} frames)",
                    min,
                    max,
                    frames,
                    requested
                );
            }
            SupportedBufferSize::Unknown => {
                tracing::info!(
                    "Device buffer size range unknown; requesting Fixed({}) (never use BufferSize::Default — see CPAL buffer size docs)",
                    requested
                );
            }
        }
    }

    /// Sample rates to try, in order: the selected one, then the device's shared mix rate.
    ///
    /// WASAPI answers `S_FALSE` ("usable only with format conversion") for anything that is
    /// not exactly its shared mix format, and CPAL counts only `S_OK` as supported — even when
    /// `supported_output_configs()` advertises a range containing the rate. A device mixing at
    /// 44.1 kHz therefore rejects 48 kHz on open while listing 44.1..=48 kHz. The mix rate is
    /// what `default_output_config()` reports, so that is the working fallback.
    fn rate_candidates(requested: u32, device_default: Option<u32>) -> Vec<u32> {
        let mut rates = vec![requested];
        if let Some(default) = device_default {
            if default != requested {
                rates.push(default);
            }
        }
        rates
    }

    /// Build (and immediately drop) a silent stream to learn whether the device accepts it.
    ///
    /// The only reliable answer for shared mode. A separate stub is required because CPAL
    /// consumes the real callback on a failed build, so a failed open cannot be retried with
    /// the same callback.
    fn stream_supported(device: &cpal::Device, config: &StreamConfig) -> bool {
        device
            .build_output_stream::<f32, _, _>(
                *config,
                |data: &mut [f32], _: &cpal::OutputCallbackInfo| data.fill(0.0),
                |_| {},
                None,
            )
            .is_ok()
    }

    fn device_candidates(&self, preferred: &DeviceId) -> Vec<cpal::Device> {
        let mut candidates = Vec::new();
        let mut seen = HashSet::new();

        let mut push = |device: cpal::Device| {
            if let Ok(id) = Self::device_id(&device) {
                if seen.insert(id.as_str().to_string()) {
                    candidates.push(device);
                }
            }
        };

        if let Ok(device) = self.find_device_by_id(preferred) {
            push(device);
        }
        if let Some(device) = self.host.default_output_device() {
            push(device);
        }
        if let Ok(devices) = self.host.output_devices() {
            for device in devices {
                push(device);
            }
        }

        candidates
    }

    fn max_output_channels(configs: &[SupportedStreamConfigRange]) -> u16 {
        configs
            .iter()
            .map(|config| config.channels())
            .max()
            .unwrap_or(0)
    }

    /// Pick the stream format for `params` from a device's supported ranges.
    ///
    /// Three constraints decide it, and getting any of them wrong makes
    /// `build_output_stream` fail inside CPAL with "not supported in shared mode":
    ///
    /// - the sample format must be `f32`, because `AudioCallback::render` hands the
    ///   device `f32` samples (CPAL builds a typed `WAVEFORMATEXTENSIBLE` for it);
    /// - the requested channel count;
    /// - the requested sample rate, falling back to the device's default rate. WASAPI
    ///   shared mode is locked to the engine mix format, so a virtual/RDP device that
    ///   mixes at 44.1 kHz can never run 48 kHz; the engine follows the device rather
    ///   than refusing to start.
    fn pick_device_format(
        supported: &[SupportedStreamConfigRange],
        default_rate: Option<u32>,
        requested_rate: u32,
        channels: u16,
    ) -> Result<DeviceFormat> {
        let candidates: Vec<&SupportedStreamConfigRange> = supported
            .iter()
            .filter(|config| config.sample_format() == SampleFormat::F32)
            .filter(|config| config.channels() == channels)
            .collect();
        if candidates.is_empty() {
            let formats: Vec<String> = supported
                .iter()
                .map(|config| format!("{:?}x{}ch", config.sample_format(), config.channels()))
                .collect();
            return Err(anyhow::anyhow!(
                "no f32 {channels}-channel output config (device offers: {})",
                if formats.is_empty() {
                    "nothing".to_string()
                } else {
                    formats.join(", ")
                }
            ));
        }

        let contains = |range: &SupportedStreamConfigRange, rate: u32| {
            range.min_sample_rate() <= rate && range.max_sample_rate() >= rate
        };

        if let Some(range) = candidates
            .iter()
            .find(|range| contains(range, requested_rate))
        {
            return Ok(DeviceFormat {
                range: **range,
                sample_rate: requested_rate,
            });
        }

        // The requested rate is unavailable: use the device default when it fits,
        // otherwise the bound of some range closest to what was asked for.
        let sample_rate = default_rate
            .filter(|rate| candidates.iter().any(|range| contains(range, *rate)))
            .or_else(|| {
                candidates
                    .iter()
                    .map(|range| {
                        requested_rate.clamp(range.min_sample_rate(), range.max_sample_rate())
                    })
                    .min_by_key(|rate| requested_rate.abs_diff(*rate))
            })
            .ok_or_else(|| {
                anyhow::anyhow!("device cannot run {requested_rate} Hz and reports no usable rate")
            })?;
        let range = candidates
            .iter()
            .find(|range| contains(range, sample_rate))
            .copied()
            .ok_or_else(|| anyhow::anyhow!("no supported config contains {sample_rate} Hz"))?;
        Ok(DeviceFormat {
            range: *range,
            sample_rate,
        })
    }

    fn select_stream_config(device: &cpal::Device, params: &StreamParams) -> Result<DeviceFormat> {
        let supported: Vec<_> = device.supported_output_configs()?.collect();
        let default_config = device.default_output_config().ok();
        let default_rate = default_config.as_ref().map(|config| config.sample_rate());

        let format = Self::pick_device_format(
            &supported,
            default_rate,
            params.sample_rate,
            params.channels,
        )?;

        Self::log_device_formats(&supported, default_config.as_ref(), &format, params);

        Ok(format)
    }

    fn resolve_open_target(
        &self,
        preferred: &DeviceId,
        params: &StreamParams,
    ) -> Result<(cpal::Device, DeviceFormat)> {
        let mut last_error: Option<anyhow::Error> = None;

        for cpal_device in self.device_candidates(preferred) {
            let device_name = Self::device_name(&cpal_device);
            match Self::select_stream_config(&cpal_device, params) {
                Ok(format) => return Ok((cpal_device, format)),
                Err(error) => {
                    tracing::warn!(
                        "Skipping output device '{}' for stream setup: {}",
                        device_name,
                        error
                    );
                    last_error = Some(error);
                }
            }
        }

        Err(last_error.unwrap_or_else(|| anyhow::anyhow!("No output device available")))
    }

    /// Log what the device can do and what was chosen, so a rejected stream is
    /// diagnosable from `mixar.log` instead of only from CPAL's error.
    fn log_device_formats(
        supported: &[SupportedStreamConfigRange],
        default_config: Option<&SupportedStreamConfig>,
        chosen: &DeviceFormat,
        params: &StreamParams,
    ) {
        if let Some(config) = default_config {
            tracing::info!(
                "Device default output: {} Hz, {} ch, {:?}, buffer {:?}",
                config.sample_rate(),
                config.channels(),
                config.sample_format(),
                config.config().buffer_size
            );
        }
        let ranges: Vec<String> = supported
            .iter()
            .map(|config| {
                format!(
                    "{}..{}Hz {:?}x{}ch",
                    config.min_sample_rate(),
                    config.max_sample_rate(),
                    config.sample_format(),
                    config.channels()
                )
            })
            .collect();
        tracing::info!("Device supported output configs: {}", ranges.join(", "));

        if chosen.sample_rate != params.sample_rate {
            tracing::warn!(
                requested_hz = params.sample_rate,
                device_hz = chosen.sample_rate,
                "device cannot run the configured sample rate; the engine will follow the device"
            );
        }
        Self::log_device_buffer_limits_frames(chosen.range.buffer_size(), params.frames_per_buffer);
    }
}

impl AudioBackend for CpalBackend {
    fn name(&self) -> &'static str {
        "cpal"
    }

    fn list_output_devices(&self) -> Result<Vec<DeviceInfo>> {
        let mut devices = Vec::new();

        let default_id = self
            .host
            .default_output_device()
            .and_then(|d| Self::device_id(&d).ok());

        let output_devices = self.host.output_devices()?;
        for device in output_devices {
            let id = match Self::device_id(&device) {
                Ok(i) => i,
                Err(_) => continue,
            };
            let device_name = Self::device_name(&device);
            if !Self::should_list_output_device(id.as_str(), &device_name) {
                continue;
            }
            let is_default = default_id
                .as_ref()
                .is_some_and(|d| d.as_str() == id.as_str());

            let supported_configs: Vec<_> = match device.supported_output_configs() {
                Ok(configs) => configs.collect(),
                Err(_) => continue,
            };
            if supported_configs.is_empty() {
                continue;
            }

            let max_channels = Self::max_output_channels(&supported_configs);
            if max_channels == 0 {
                continue;
            }

            let mut sample_rates = vec![44100, 48000];
            for config in &supported_configs {
                sample_rates.push(config.min_sample_rate());
                sample_rates.push(config.max_sample_rate());
            }
            sample_rates.sort();
            sample_rates.dedup();

            let device_info =
                DeviceInfo::new(id, device_name, max_channels, sample_rates, is_default);
            devices.push(device_info);
        }

        Ok(devices)
    }

    fn open_output_stream(
        &mut self,
        device: &DeviceId,
        params: &StreamParams,
        callback: Box<dyn AudioCallback>,
    ) -> Result<Box<dyn AudioStream>> {
        let (cpal_device, format) = self.resolve_open_target(device, params)?;
        let device_name = Self::device_name(&cpal_device);
        let supported_config = format.range.with_sample_rate(format.sample_rate);
        if supported_config.channels() != params.channels {
            return Err(anyhow::anyhow!(
                "Device '{}' opened with {} channels but {} channels were requested",
                device_name,
                supported_config.channels(),
                params.channels
            ));
        }

        // The device may only accept its own period (WASAPI shared mode); engine-core keeps the
        // DSP chunk at the configured size and lets the ring buffer absorb the difference.
        let requested_frames = params.frames_per_buffer;
        let device_frames = Self::negotiate_frames(format.range.buffer_size(), requested_frames);

        // Picking by advertised range is not enough: probe the selected rate, then the device's
        // mix rate, and open at the first the device actually accepts.
        let device_default_rate = cpal_device
            .default_output_config()
            .ok()
            .map(|config| config.sample_rate());
        let mut candidates = Self::rate_candidates(format.sample_rate, device_default_rate);
        let actual_sample_rate = if candidates.len() == 1 {
            candidates.remove(0)
        } else {
            tracing::info!(
                device = %device_name,
                rates = ?candidates,
                "probing sample rates: the device mix rate differs from the configured one"
            );
            let mut accepted = None;
            for rate in &candidates {
                let probe = StreamConfig {
                    channels: supported_config.channels(),
                    sample_rate: *rate,
                    buffer_size: BufferSize::Fixed(device_frames),
                };
                if Self::stream_supported(&cpal_device, &probe) {
                    accepted = Some(*rate);
                    break;
                }
                tracing::warn!(
                    device = %device_name,
                    rate_hz = rate,
                    "device rejected this sample rate in shared mode; trying the device mix rate"
                );
            }
            accepted.ok_or_else(|| {
                anyhow::anyhow!(
                    "Device '{}' accepted none of the sample rates {:?} for {:?} {} ch, {} frames/callback",
                    device_name,
                    candidates,
                    supported_config.sample_format(),
                    supported_config.channels(),
                    device_frames
                )
            })?
        };
        if actual_sample_rate != params.sample_rate {
            tracing::warn!(
                device = %device_name,
                requested_hz = params.sample_rate,
                actual_hz = actual_sample_rate,
                "device cannot open the configured sample rate in shared mode; opening at its mix rate and running the engine there"
            );
        }

        tracing::info!(
            "Opening CPAL output on '{}' ({} Hz {:?}, {} channels, {} frames/callback, configured {} Hz / {} frames)",
            device_name,
            actual_sample_rate,
            supported_config.sample_format(),
            supported_config.channels(),
            device_frames,
            params.sample_rate,
            requested_frames
        );

        let channels = supported_config.channels();
        let last_callback_frames = Arc::new(AtomicU32::new(0));

        let config = StreamConfig {
            channels: supported_config.channels(),
            sample_rate: actual_sample_rate,
            buffer_size: BufferSize::Fixed(device_frames),
        };
        let frames = Arc::clone(&last_callback_frames);
        let mut callback = callback;
        let stream = cpal_device
            .build_output_stream(
                config,
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    let n = data.len() / channels as usize;
                    frames.store(n as u32, Ordering::Relaxed);
                    data.fill(0.0);
                    callback.render(data, n as u32, actual_sample_rate);
                },
                |err| {
                    eprintln!("Audio stream error: {}", err);
                },
                None,
            )
            .map_err(|error| {
                // CPAL reports format/rate/channel rejections as "not supported in shared
                // mode" and buffer rejections separately, so name the format that was tried
                // instead of guessing which part was refused.
                anyhow::anyhow!(
                    "Device '{}' rejected {} Hz, {:?}, {} ch, {} frames/callback: {}",
                    device_name,
                    actual_sample_rate,
                    supported_config.sample_format(),
                    supported_config.channels(),
                    device_frames,
                    error
                )
            })?;

        // The callback's own frame count is authoritative: WASAPI shared mode always delivers its
        // device period regardless of what was requested, so a differing grant is not an error.
        let granted_buffer = stream.buffer_size().unwrap_or(device_frames);
        if granted_buffer != device_frames {
            tracing::info!(
                "CPAL granted {} frames on '{}' for a {} frame request",
                granted_buffer,
                device_name,
                device_frames
            );
        }

        tracing::info!(
            "CPAL stream opened on '{}' with buffer size {} frames",
            device_name,
            granted_buffer
        );

        Ok(Box::new(CpalStream {
            stream,
            frames_per_buffer: granted_buffer,
            sample_rate: actual_sample_rate,
            last_callback_frames,
        }))
    }
}

/// CPAL stream implementation
struct CpalStream {
    stream: Stream,
    frames_per_buffer: u32,
    sample_rate: u32,
    /// Actual frames per callback (driver may differ from requested)
    last_callback_frames: Arc<AtomicU32>,
}

// Implement Send manually to work around CPAL's Send issues
unsafe impl Send for CpalStream {}

impl AudioStream for CpalStream {
    fn start(&mut self) -> Result<()> {
        self.stream.play()?;
        Ok(())
    }

    fn stop(&mut self) -> Result<()> {
        // CPAL streams are automatically stopped when dropped
        Ok(())
    }

    fn actual_buffer_size(&self) -> Option<u32> {
        // Once the device has called back, its own frame count is the truth: WASAPI shared mode
        // delivers the device period regardless of the requested size. Before the first callback
        // this reports what was requested/granted.
        let live = self.last_callback_frames.load(Ordering::Relaxed);
        if live > 0 {
            return Some(live);
        }
        if let Ok(size) = self.stream.buffer_size() {
            return Some(size);
        }
        Some(self.frames_per_buffer)
    }

    fn callback_frames_atomic(&self) -> Option<Arc<AtomicU32>> {
        Some(Arc::clone(&self.last_callback_frames))
    }

    fn actual_sample_rate(&self) -> Option<u32> {
        Some(self.sample_rate)
    }

    fn actual_latency(&self) -> Option<Duration> {
        let buffer_duration =
            Duration::from_secs_f64(self.frames_per_buffer as f64 / self.sample_rate as f64);
        Some(buffer_duration)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cpal::SampleFormat;

    fn test_config_range(sample_rate: u32, channels: u16) -> SupportedStreamConfigRange {
        SupportedStreamConfigRange::new(
            channels,
            sample_rate,
            sample_rate * 2,
            SupportedBufferSize::Range { min: 256, max: 512 },
            SampleFormat::F32,
        )
    }

    #[test]
    fn max_output_channels_reports_highest_supported() {
        let stereo = test_config_range(48_000, 2);
        let quad = test_config_range(48_000, 4);
        let configs = [stereo, quad];
        assert_eq!(CpalBackend::max_output_channels(&configs), 4);
    }

    fn range(
        channels: u16,
        min_rate: u32,
        max_rate: u32,
        format: SampleFormat,
    ) -> SupportedStreamConfigRange {
        SupportedStreamConfigRange::new(
            channels,
            min_rate,
            max_rate,
            SupportedBufferSize::Range { min: 256, max: 512 },
            format,
        )
    }

    #[test]
    fn pick_device_format_selects_exact_channels() {
        let configs = [
            range(2, 48_000, 96_000, SampleFormat::F32),
            range(4, 48_000, 96_000, SampleFormat::F32),
        ];

        let picked = CpalBackend::pick_device_format(&configs, None, 48_000, 4).unwrap();
        assert_eq!(picked.range.channels(), 4);
        assert_eq!(picked.sample_rate, 48_000);

        let picked = CpalBackend::pick_device_format(&configs, None, 48_000, 2).unwrap();
        assert_eq!(picked.range.channels(), 2);
    }

    #[test]
    fn pick_device_format_names_available_channels_instead_of_the_request() {
        let configs = [range(2, 48_000, 96_000, SampleFormat::F32)];

        let error = CpalBackend::pick_device_format(&configs, None, 48_000, 4).unwrap_err();
        let message = error.to_string();
        assert!(message.contains("f32 4-channel"), "{message}");
        assert!(message.contains("2ch"), "{message}");
    }

    /// WASAPI shared mode builds a typed `WAVEFORMATEXTENSIBLE` for the `f32` callback, so a
    /// range in another sample format must never be selected — that is what produced
    /// "not supported in shared mode" while the enumeration looked fine.
    #[test]
    fn pick_device_format_requires_f32() {
        let configs = [
            range(2, 44_100, 48_000, SampleFormat::I16),
            range(2, 44_100, 48_000, SampleFormat::U16),
        ];

        let error = CpalBackend::pick_device_format(&configs, None, 48_000, 2).unwrap_err();
        assert!(error.to_string().contains("no f32 2-channel"), "{error}");
    }

    #[test]
    fn pick_device_format_falls_back_to_the_device_default_rate() {
        // A virtual/RDP device mixing at 44.1 kHz can never run 48 kHz in shared mode.
        let configs = [range(2, 44_100, 44_100, SampleFormat::F32)];

        let picked = CpalBackend::pick_device_format(&configs, Some(44_100), 48_000, 2).unwrap();
        assert_eq!(picked.sample_rate, 44_100);
    }

    #[test]
    fn pick_device_format_falls_back_to_the_nearest_bound_without_a_default() {
        let configs = [range(2, 22_050, 32_000, SampleFormat::F32)];

        let picked = CpalBackend::pick_device_format(&configs, None, 48_000, 2).unwrap();
        assert_eq!(picked.sample_rate, 32_000);
    }

    #[test]
    fn rate_candidates_try_the_requested_rate_first() {
        assert_eq!(CpalBackend::rate_candidates(48_000, None), vec![48_000]);
        assert_eq!(
            CpalBackend::rate_candidates(48_000, Some(48_000)),
            vec![48_000]
        );
    }

    #[test]
    fn rate_candidates_add_the_device_mix_rate_as_fallback() {
        // A device mixing at 44.1 kHz rejects 48 kHz in shared mode; 44.1 is the retry.
        assert_eq!(
            CpalBackend::rate_candidates(48_000, Some(44_100)),
            vec![48_000, 44_100]
        );
    }

    #[test]
    fn negotiate_frames_keeps_a_supported_request() {
        let supported = SupportedBufferSize::Range { min: 64, max: 2048 };
        assert_eq!(CpalBackend::negotiate_frames(&supported, 512), 512);
    }

    #[test]
    fn negotiate_frames_clamps_to_a_fixed_device_period() {
        // WASAPI shared mode advertises its own period as `Range { min == max }`, so 512 can
        // never be granted there; the engine must still be able to open the device.
        let supported = SupportedBufferSize::Range { min: 480, max: 480 };
        assert_eq!(CpalBackend::negotiate_frames(&supported, 512), 480);
    }

    #[test]
    fn negotiate_frames_clamps_below_range_minimum() {
        let supported = SupportedBufferSize::Range {
            min: 1024,
            max: 4096,
        };
        assert_eq!(CpalBackend::negotiate_frames(&supported, 512), 1024);
    }

    #[test]
    fn negotiate_frames_keeps_request_when_range_is_unknown() {
        assert_eq!(
            CpalBackend::negotiate_frames(&SupportedBufferSize::Unknown, 512),
            512
        );
    }

    #[test]
    fn filters_pipewire_sink_default_and_unknown_streams() {
        assert!(!CpalBackend::should_list_output_device(
            "cpal:pipewire:sink_default",
            "sink_default"
        ));
        assert!(!CpalBackend::should_list_output_device(
            "cpal:pipewire:mixar",
            "unknown"
        ));
        assert!(!CpalBackend::should_list_output_device(
            "cpal:pipewire:mixar",
            "Unknown Device"
        ));
        assert!(CpalBackend::should_list_output_device(
            "cpal:pipewire:output_default",
            "output_default"
        ));
        assert!(CpalBackend::should_list_output_device(
            "cpal:pipewire:alsa_output.usb-HD-II",
            "HD-II"
        ));
    }

    #[test]
    fn test_cpal_backend_creation() {
        let backend = CpalBackend::new();
        assert!(backend.is_ok());
    }

    #[test]
    fn test_cpal_backend_name() {
        let backend = CpalBackend::new().unwrap();
        assert_eq!(backend.name(), "cpal");
    }

    #[test]
    fn test_cpal_device_listing() {
        let backend = CpalBackend::new().unwrap();
        let devices = backend.list_output_devices();
        assert!(devices.is_ok());

        let devices = devices.unwrap();
        if !devices.is_empty() {
            let host = create_host().unwrap();
            for device in &devices {
                assert!(
                    !device.name.eq_ignore_ascii_case("sink_default"),
                    "sink_default must not be listed"
                );
                assert!(
                    !device.name.eq_ignore_ascii_case("unknown"),
                    "unnamed stream outputs must not be listed"
                );
                assert!(
                    device.id.as_str().starts_with("cpal:"),
                    "Device ID should start with 'cpal:': {}",
                    device.id.as_str()
                );
                assert!(
                    device.max_channels >= 2,
                    "Device '{}' should report at least stereo channels",
                    device.name
                );

                let suffix = device.id.as_str().strip_prefix("cpal:").unwrap();
                let cpal_id: cpal::DeviceId = suffix.parse().unwrap();
                let cpal_device = host.device_by_id(&cpal_id).unwrap();
                let expected = cpal_device
                    .supported_output_configs()
                    .ok()
                    .into_iter()
                    .flatten()
                    .map(|config| config.channels())
                    .max()
                    .unwrap_or(0);
                assert_eq!(
                    device.max_channels, expected,
                    "max_channels mismatch for device '{}'",
                    device.name
                );

                println!(
                    "Device: {} -> ID: {} (max_channels={})",
                    device.name,
                    device.id.as_str(),
                    device.max_channels
                );
            }
        }
    }

    #[test]
    fn test_cpal_default_device() {
        let backend = CpalBackend::new().unwrap();
        let devices = backend.list_output_devices().unwrap();
        let default = devices.iter().find(|d| d.is_default).or(devices.first());
        if let Some(d) = default {
            assert!(!d.name.is_empty());
        }
    }
}
