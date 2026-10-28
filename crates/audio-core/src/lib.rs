//! Core audio traits and types for Mixar
//!
//! This crate defines the fundamental abstractions for audio backends,
//! streams, and callbacks that all audio backends must implement.
//!
//! Sample and frame types align with [dasp](https://github.com/RustAudio/dasp) (Digital Audio
//! Signal Processing): we use `f32` as the internal sample type (dasp `Sample` trait) and
//! re-export dasp's `Frame` trait and `slice` module for frame/sample conversions.

mod audio_source;
mod live_stems;
pub mod time;
mod waveform;

use std::sync::atomic::AtomicU32;
use std::sync::Arc;
use std::time::Duration;

pub use audio_source::{LoadableAudio, LoadedAudio};
pub use live_stems::{LiveStemRing, DEFAULT_LIVE_STEM_CAPACITY, LIVE_STEM_COUNT};
pub use time::{ms_to_secs, secs_to_ms};
pub use waveform::{
    compute_full_track_envelope, compute_overview_envelope, compute_peak_envelope,
    compute_spectral_envelope, compute_spectral_window, peaks_to_rgb_bytes, rgb_bytes_to_peaks,
    waveform_buckets_for_duration, waveform_buckets_for_window, SpectralPeak,
    WaveformAmplitudeMode, WaveformAnalysisConfig, WaveformChannelMode, WaveformFilterKind,
    MAX_FULL_TRACK_BUCKETS, MAX_WAVEFORM_BUCKETS, MIN_WAVEFORM_BUCKETS, OVERVIEW_SAMPLE_COUNT,
    WAVEFORM_SCHEMA_VERSION,
};

// Re-export dasp fundamentals for samples, frames, and slice conversions
pub use dasp::frame::Frame;
pub use dasp::sample::{Sample as SampleTrait, ToSample};
pub use dasp::slice;

/// Internal sample format - always 32-bit float (compatible with [dasp] Sample trait)
pub type Sample = f32;

/// Stereo frame: one sample per channel at a single time (L, R)
pub type StereoFrame = [Sample; 2];

/// How a deck lane is affected by the main crossfader.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CrossfaderAssign {
    /// Routed to crossfader side A: silent when the fader is fully at B.
    A,
    /// Bypasses the crossfader and stays at full level.
    Thru,
    /// Routed to crossfader side B: silent when the fader is fully at A.
    B,
}

/// Unique identifier for an audio device
#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct DeviceId(pub String);

impl DeviceId {
    /// Create a new device ID
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Get the device ID as a string
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<String> for DeviceId {
    fn from(id: String) -> Self {
        Self(id)
    }
}

impl From<&str> for DeviceId {
    fn from(id: &str) -> Self {
        Self(id.to_string())
    }
}

/// Inclusive span of sample rates a device can open.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SampleRateRange {
    pub min: u32,
    pub max: u32,
}

impl SampleRateRange {
    /// Create a range.
    pub fn new(min: u32, max: u32) -> Self {
        Self { min, max }
    }

    /// Whether `rate` falls inside the span.
    pub fn contains(&self, rate: u32) -> bool {
        (self.min..=self.max).contains(&rate)
    }
}

/// Information about an audio device
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct DeviceInfo {
    /// Unique device identifier
    pub id: DeviceId,
    /// Human-readable device name
    pub name: String,
    /// Maximum number of output channels
    pub max_channels: u16,
    /// Default sample rates supported by this device
    pub default_sample_rates: Vec<u32>,
    /// Rate the device mixes at when the system owns the format (WASAPI shared
    /// mode mix rate, PipeWire graph rate). `None` when the backend cannot report
    /// it; a bus without an explicit rate then has no automatic choice.
    #[serde(default)]
    pub mix_sample_rate: Option<u32>,
    /// Spans of rates the device can open. `default_sample_rates` lists their
    /// endpoints; a rate inside a span is still valid, which a UI needs to avoid
    /// discarding a rate the device accepts.
    #[serde(default)]
    pub sample_rate_ranges: Vec<SampleRateRange>,
    /// Whether this device is the system default output device
    pub is_default: bool,
}

impl DeviceInfo {
    /// Create a new device info
    pub fn new(
        id: DeviceId,
        name: String,
        max_channels: u16,
        sample_rates: Vec<u32>,
        is_default: bool,
    ) -> Self {
        Self {
            id,
            name,
            max_channels,
            default_sample_rates: sample_rates,
            mix_sample_rate: None,
            sample_rate_ranges: Vec::new(),
            is_default,
        }
    }

    /// Record the rate the system mixes this device at (see [`Self::mix_sample_rate`]).
    pub fn with_mix_sample_rate(mut self, rate: Option<u32>) -> Self {
        self.mix_sample_rate = rate;
        self
    }

    /// Record the spans of rates the device can open (see [`Self::sample_rate_ranges`]).
    pub fn with_sample_rate_ranges(mut self, ranges: Vec<SampleRateRange>) -> Self {
        self.sample_rate_ranges = ranges;
        self
    }
}

/// Parameters for opening an audio stream
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct StreamParams {
    /// Sample rate in Hz
    pub sample_rate: u32,
    /// Number of channels (e.g., 2 for stereo)
    pub channels: u16,
    /// Requested frames per buffer (e.g., 512)
    pub frames_per_buffer: u32,
    /// Hint for low-latency operation: a backend may request its smallest safe
    /// device period instead of the configured one. CPAL uses the device's
    /// minimum supported buffer when the driver reports a range (floored at 16
    /// frames unless the whole range is smaller); a device that reports no range
    /// keeps the configured size. Never changes the sample rate.
    pub low_latency: bool,
}

impl StreamParams {
    /// Create new stream parameters
    pub fn new(sample_rate: u32, channels: u16, frames_per_buffer: u32, low_latency: bool) -> Self {
        Self {
            sample_rate,
            channels,
            frames_per_buffer,
            low_latency,
        }
    }

    /// Get the duration of one buffer in seconds
    pub fn buffer_duration(&self) -> Duration {
        let seconds = self.frames_per_buffer as f64 / self.sample_rate as f64;
        Duration::from_secs_f64(seconds)
    }
}

/// Audio callback trait for real-time audio processing
///
/// This trait is implemented by the engine to provide audio data
/// to the backend. The backend will call `render()` from the audio thread.
pub trait AudioCallback: Send {
    /// Fill the output buffer with interleaved samples
    ///
    /// # Arguments
    /// * `out` - Output buffer to fill with interleaved samples
    /// * `frames` - Number of frames to render
    /// * `sample_rate` - Current sample rate
    ///
    /// # Notes
    /// - `out.len()` should equal `frames * channels`
    /// - This method is called from the audio thread and must be real-time safe
    /// - No heap allocations or blocking operations should be performed
    fn render(&mut self, out: &mut [Sample], frames: u32, sample_rate: u32);
}

/// Audio stream handle for controlling playback
pub trait AudioStream: Send {
    /// Start the audio stream
    fn start(&mut self) -> anyhow::Result<()>;

    /// Stop the audio stream
    fn stop(&mut self) -> anyhow::Result<()>;

    /// Get the actual buffer size granted by the backend
    /// Returns None if not available or not started
    fn actual_buffer_size(&self) -> Option<u32>;

    /// Optional: atomic updated by the callback with frames per call.
    /// Lets the producer match the real callback size before the first callback runs (0 = use fallback).
    fn callback_frames_atomic(&self) -> Option<Arc<AtomicU32>> {
        None
    }

    /// Get the actual sample rate granted by the backend
    /// Returns None if not available or not started
    fn actual_sample_rate(&self) -> Option<u32>;

    /// Get the actual latency of the stream
    /// Returns None if not available or not started
    fn actual_latency(&self) -> Option<Duration>;

    /// Device the stream actually opened on, when the backend can report it.
    ///
    /// Backends that search for a usable device may open a different one than
    /// requested; callers use this to notice that instead of assuming the request
    /// was honoured.
    fn device_id(&self) -> Option<&DeviceId> {
        None
    }
}

/// Audio backend trait for different audio systems
///
/// This trait defines the interface that all audio backends must implement.
/// Backends are selected at runtime and compiled into the binary.
pub trait AudioBackend: Send + Sync {
    /// Get the name of this backend
    fn name(&self) -> &'static str;

    /// List all available output devices. Each device has `is_default` set if it is the system default.
    fn list_output_devices(&self) -> anyhow::Result<Vec<DeviceInfo>>;

    /// Open an output stream with the given parameters
    ///
    /// # Arguments
    /// * `device` - Device to open the stream on
    /// * `params` - Stream parameters (sample rate, channels, buffer size)
    /// * `callback` - Audio callback to provide samples
    ///
    /// # Returns
    /// A boxed AudioStream that can be used to control playback
    fn open_output_stream(
        &mut self,
        device: &DeviceId,
        params: &StreamParams,
        callback: Box<dyn AudioCallback>,
    ) -> anyhow::Result<Box<dyn AudioStream>>;
}

/// Bus identifier for audio routing
#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct BusId(pub String);

impl BusId {
    /// Create a new bus ID
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Get the bus ID as a string
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<String> for BusId {
    fn from(id: String) -> Self {
        Self(id)
    }
}

impl From<&str> for BusId {
    fn from(id: &str) -> Self {
        Self(id.to_string())
    }
}

/// Whether a bus maps to a stereo pair or a single mono device channel.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChannelMode {
    /// Map bus L/R to two distinct 1-based device channels.
    #[default]
    Stereo,
    /// Fold bus L/R to mono on a single 1-based device channel (`left`).
    Mono,
}

/// Channel mapping for a bus
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ChannelMapping {
    /// Left channel index (1-based). For [`ChannelMode::Mono`], the mono output channel.
    pub left: u16,
    /// Right channel index (1-based). Ignored for [`ChannelMode::Mono`] (kept for serde compat).
    pub right: u16,
    /// Stereo pair vs mono fold. Defaults to stereo for older configs without this field.
    #[serde(default)]
    pub mode: ChannelMode,
}

impl ChannelMapping {
    /// Create a stereo channel mapping (legacy helper name).
    pub fn new(left: u16, right: u16) -> Self {
        Self::stereo(left, right)
    }

    /// Stereo mapping onto two distinct 1-based device channels.
    pub fn stereo(left: u16, right: u16) -> Self {
        Self {
            left,
            right,
            mode: ChannelMode::Stereo,
        }
    }

    /// Mono mapping: stereo bus is folded onto one 1-based device channel.
    pub fn mono(channel: u16) -> Self {
        Self {
            left: channel,
            right: channel,
            mode: ChannelMode::Mono,
        }
    }

    /// Whether this mapping is mono.
    pub fn is_mono(&self) -> bool {
        matches!(self.mode, ChannelMode::Mono)
    }

    /// Convert stereo L/R to 0-based indices. For mono, both values are the same channel.
    pub fn to_zero_based(&self) -> (usize, usize) {
        match self.mode {
            ChannelMode::Mono => {
                let ch = (self.left.saturating_sub(1)) as usize;
                (ch, ch)
            }
            ChannelMode::Stereo => ((self.left - 1) as usize, (self.right - 1) as usize),
        }
    }

    /// 0-based device channels occupied by this mapping.
    pub fn occupied_zero_based(&self) -> Vec<usize> {
        match self.mode {
            ChannelMode::Mono => vec![(self.left.saturating_sub(1)) as usize],
            ChannelMode::Stereo => {
                let (l, r) = self.to_zero_based();
                vec![l, r]
            }
        }
    }

    /// Highest 1-based channel index used.
    pub fn highest_channel(&self) -> u16 {
        match self.mode {
            ChannelMode::Mono => self.left,
            ChannelMode::Stereo => self.left.max(self.right),
        }
    }
}

/// Bus configuration for audio routing
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct BusConfig {
    /// Bus identifier
    pub id: BusId,
    /// Human-readable bus name
    pub name: String,
    /// Target device for this bus
    pub device: DeviceId,
    /// Channel mapping for this bus
    pub channels: ChannelMapping,
    /// Output rate for this bus's device stream. `None` follows the device's
    /// mix rate ([`DeviceInfo::mix_sample_rate`]). Buses sharing a device must
    /// agree, since they share one stream.
    #[serde(default)]
    pub sample_rate: Option<u32>,
    /// Ask the backend for a shorter stream path on this bus's device. A hint:
    /// the backend may ignore it, and a device stream shared by several buses
    /// runs with low latency when any of them asks for it.
    #[serde(default)]
    pub low_latency: bool,
}

impl BusConfig {
    /// Create a new bus configuration
    pub fn new(id: BusId, name: String, device: DeviceId, channels: ChannelMapping) -> Self {
        Self {
            id,
            name,
            device,
            channels,
            sample_rate: None,
            low_latency: false,
        }
    }

    /// Set the output rate for this bus (`None` follows the device mix rate).
    pub fn with_sample_rate(mut self, sample_rate: Option<u32>) -> Self {
        self.sample_rate = sample_rate;
        self
    }

    /// Ask the backend for a shorter path on this bus's device.
    pub fn with_low_latency(mut self, low_latency: bool) -> Self {
        self.low_latency = low_latency;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_id() {
        let id = DeviceId::new("test-device");
        assert_eq!(id.as_str(), "test-device");
    }

    #[test]
    fn test_stream_params() {
        let params = StreamParams::new(48000, 2, 512, false);
        assert_eq!(params.sample_rate, 48000);
        assert_eq!(params.channels, 2);
        assert_eq!(params.frames_per_buffer, 512);
        assert!(!params.low_latency);
    }

    #[test]
    fn channel_mapping_mono_occupies_one_channel() {
        let mapping = ChannelMapping::mono(3);
        assert!(mapping.is_mono());
        assert_eq!(mapping.mode, ChannelMode::Mono);
        assert_eq!(mapping.occupied_zero_based(), vec![2]);
        assert_eq!(mapping.highest_channel(), 3);
        assert_eq!(mapping.to_zero_based(), (2, 2));
    }

    #[test]
    fn channel_mapping_new_is_stereo() {
        let mapping = ChannelMapping::new(1, 2);
        assert_eq!(mapping.mode, ChannelMode::Stereo);
        assert_eq!(mapping.occupied_zero_based(), vec![0, 1]);
    }

    #[test]
    fn test_buffer_duration() {
        let params = StreamParams::new(48000, 2, 512, false);
        let duration = params.buffer_duration();
        let expected_ms = (512.0 / 48000.0 * 1000.0) as u64;
        assert_eq!(duration.as_millis(), expected_ms as u128);
    }
}
