//! Core audio traits and types for rust-dj-engine
//!
//! This crate defines the fundamental abstractions for audio backends,
//! streams, and callbacks that all audio backends must implement.
//!
//! Sample and frame types align with [dasp](https://github.com/RustAudio/dasp) (Digital Audio
//! Signal Processing): we use `f32` as the internal sample type (dasp `Sample` trait) and
//! re-export dasp's `Frame` trait and `slice` module for frame/sample conversions.

use std::sync::atomic::AtomicU32;
use std::sync::Arc;
use std::time::Duration;

// Re-export dasp fundamentals for samples, frames, and slice conversions
pub use dasp::frame::Frame;
pub use dasp::sample::{Sample as SampleTrait, ToSample};
pub use dasp::slice;

/// Internal sample format - always 32-bit float (compatible with [dasp] Sample trait)
pub type Sample = f32;

/// Stereo frame: one sample per channel at a single time (L, R)
pub type StereoFrame = [Sample; 2];

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
}

impl DeviceInfo {
    /// Create a new device info
    pub fn new(id: DeviceId, name: String, max_channels: u16, sample_rates: Vec<u32>) -> Self {
        Self {
            id,
            name,
            max_channels,
            default_sample_rates: sample_rates,
        }
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
    /// Hint for low-latency operation
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
}

/// Audio backend trait for different audio systems
///
/// This trait defines the interface that all audio backends must implement.
/// Backends are selected at runtime and compiled into the binary.
pub trait AudioBackend: Send + Sync {
    /// Get the name of this backend
    fn name(&self) -> &'static str;

    /// List all available output devices
    fn list_output_devices(&self) -> anyhow::Result<Vec<DeviceInfo>>;

    /// Get the default output device
    fn default_output_device(&self) -> anyhow::Result<DeviceInfo>;

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

/// Channel mapping for a bus
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ChannelMapping {
    /// Left channel index (1-based)
    pub left: u16,
    /// Right channel index (1-based)
    pub right: u16,
}

impl ChannelMapping {
    /// Create a new channel mapping
    pub fn new(left: u16, right: u16) -> Self {
        Self { left, right }
    }

    /// Convert to 0-based indices
    pub fn to_zero_based(&self) -> (usize, usize) {
        ((self.left - 1) as usize, (self.right - 1) as usize)
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
}

impl BusConfig {
    /// Create a new bus configuration
    pub fn new(id: BusId, name: String, device: DeviceId, channels: ChannelMapping) -> Self {
        Self {
            id,
            name,
            device,
            channels,
        }
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
    fn test_buffer_duration() {
        let params = StreamParams::new(48000, 2, 512, false);
        let duration = params.buffer_duration();
        let expected_ms = (512.0 / 48000.0 * 1000.0) as u64;
        assert_eq!(duration.as_millis(), expected_ms as u128);
    }

    #[test]
    fn test_channel_mapping() {
        let mapping = ChannelMapping::new(1, 2);
        assert_eq!(mapping.to_zero_based(), (0, 1));
    }
}
