//! CPAL backend implementation
//!
//! This backend uses the CPAL library for cross-platform audio I/O.
//! On Linux and BSD, the native PipeWire host is preferred when available.

use anyhow::Result;
use audio_core::{AudioBackend, AudioCallback, AudioStream, DeviceId, DeviceInfo, StreamParams};
use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    BufferSize, Host, Stream, StreamConfig, SupportedBufferSize, SupportedStreamConfigRange,
};
use std::collections::HashSet;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

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
                log::info!("Using CPAL PipeWire host");
                return Ok(host);
            }
            Err(e) => {
                log::warn!("PipeWire host unavailable ({e}), falling back to default CPAL host");
            }
        }
    }

    Ok(cpal::default_host())
}

/// CPAL backend implementation
pub struct CpalBackend {
    host: Host,
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

    /// Validate requested buffer size against CPAL-reported device limits.
    fn validate_buffer_size(
        device: &cpal::Device,
        config_range: &SupportedStreamConfigRange,
        requested: u32,
    ) -> Result<()> {
        if let Ok(default) = device.default_output_config() {
            log::info!(
                "Device default output buffer size: {:?}",
                default.config().buffer_size
            );
        }

        match config_range.buffer_size() {
            SupportedBufferSize::Range { min, max } => {
                log::info!(
                    "Device supported buffer size range: {}..={} frames",
                    min,
                    max
                );
                if requested < *min || requested > *max {
                    return Err(anyhow::anyhow!(
                        "Requested buffer size {} frames is outside device range {}..={} frames",
                        requested,
                        min,
                        max
                    ));
                }
            }
            SupportedBufferSize::Unknown => {
                log::info!(
                    "Device buffer size range unknown; requesting Fixed({}) (never use BufferSize::Default — see CPAL buffer size docs)",
                    requested
                );
            }
        }

        Ok(())
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

    fn select_stream_config(
        device: &cpal::Device,
        params: &StreamParams,
    ) -> Result<SupportedStreamConfigRange> {
        let desired_sample_rate = params.sample_rate;
        let supported_configs: Vec<_> = device.supported_output_configs()?.collect();

        let matching_configs: Vec<_> = supported_configs
            .iter()
            .filter(|config| {
                config.min_sample_rate() <= desired_sample_rate
                    && config.max_sample_rate() >= desired_sample_rate
            })
            .collect();

        let config_range = matching_configs
            .iter()
            .find(|config| config.channels() == 2)
            .or_else(|| matching_configs.first())
            .copied()
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "No supported config found for sample rate {}",
                    params.sample_rate
                )
            })?;

        Self::validate_buffer_size(device, &config_range, params.frames_per_buffer)?;

        Ok(*config_range)
    }

    fn resolve_open_target(
        &self,
        preferred: &DeviceId,
        params: &StreamParams,
    ) -> Result<(cpal::Device, SupportedStreamConfigRange)> {
        let mut last_error: Option<anyhow::Error> = None;

        for cpal_device in self.device_candidates(preferred) {
            let device_name = Self::device_name(&cpal_device);
            match Self::select_stream_config(&cpal_device, params) {
                Ok(config_range) => return Ok((cpal_device, config_range)),
                Err(error) => {
                    log::warn!(
                        "Skipping output device '{}' for stream setup: {}",
                        device_name,
                        error
                    );
                    last_error = Some(error);
                }
            }
        }

        Err(last_error.unwrap_or_else(|| {
            anyhow::anyhow!("No output device available")
        }))
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
            let is_default = default_id.as_ref().is_some_and(|d| d.as_str() == id.as_str());

            // Get supported sample rates (cpal 0.18: SampleRate is u32)
            let mut sample_rates = vec![44100, 48000];
            if let Ok(configs) = device.supported_output_configs() {
                for config in configs {
                    sample_rates.push(config.min_sample_rate());
                    sample_rates.push(config.max_sample_rate());
                }
            } else {
                continue;
            }
            sample_rates.sort();
            sample_rates.dedup();

            let device_info = DeviceInfo::new(id, device_name, 2, sample_rates, is_default);
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
        let desired_sample_rate = params.sample_rate;
        let (cpal_device, config_range) = self.resolve_open_target(device, params)?;
        let device_name = Self::device_name(&cpal_device);
        let supported_config = config_range.with_sample_rate(desired_sample_rate);
        let actual_sample_rate = supported_config.sample_rate();
        if actual_sample_rate != desired_sample_rate {
            return Err(anyhow::anyhow!(
                "Device opened at {} Hz but {} Hz was requested",
                actual_sample_rate,
                desired_sample_rate
            ));
        }

        log::info!(
            "Opening CPAL output on '{}' ({} Hz, {} channels, buffer size: {})",
            device_name,
            actual_sample_rate,
            supported_config.channels(),
            params.frames_per_buffer
        );

        let channels = supported_config.channels();
        let frames_per_buffer = params.frames_per_buffer;
        let callback_arc = Arc::new(Mutex::new(callback));
        let last_callback_frames = Arc::new(AtomicU32::new(0));

        let build_stream = |buffer_size: BufferSize| {
            let config = StreamConfig {
                channels: supported_config.channels(),
                sample_rate: actual_sample_rate,
                buffer_size,
            };
            let arc = Arc::clone(&callback_arc);
            let frames = Arc::clone(&last_callback_frames);
            cpal_device.build_output_stream(
                config,
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    let n = data.len() / channels as usize;
                    frames.store(n as u32, Ordering::Relaxed);
                    if let Ok(mut cb) = arc.lock() {
                        cb.render(data, n as u32, actual_sample_rate);
                    }
                },
                |err| {
                    eprintln!("Audio stream error: {}", err);
                },
                None,
            )
        };

        let stream = build_stream(BufferSize::Fixed(params.frames_per_buffer)).map_err(|error| {
            anyhow::anyhow!(
                "Device does not support fixed buffer size of {} frames: {} (do not use BufferSize::Default — see CPAL buffer size docs)",
                params.frames_per_buffer,
                error
            )
        })?;

        let granted_buffer = stream.buffer_size().unwrap_or(frames_per_buffer);
        if granted_buffer != frames_per_buffer {
            return Err(anyhow::anyhow!(
                "CPAL granted stream buffer size {} frames but {} frames were requested (BufferSize::Default causes fast-forward playback — use Fixed)",
                granted_buffer,
                frames_per_buffer
            ));
        }

        log::info!(
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
            // Test that device IDs use the new format
            for device in &devices {
                assert!(
                    device.id.as_str().starts_with("cpal:"),
                    "Device ID should start with 'cpal:': {}",
                    device.id.as_str()
                );
                println!("Device: {} -> ID: {}", device.name, device.id.as_str());
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
