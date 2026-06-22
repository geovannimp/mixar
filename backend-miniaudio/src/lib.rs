//! Miniaudio backend implementation
//!
//! This backend uses the miniaudio library for cross-platform audio I/O.
//! Currently using ep-miniaudio-sys for raw bindings to the miniaudio C library.

use anyhow::Result;
use audio_core::{AudioBackend, AudioCallback, AudioStream, DeviceId, DeviceInfo, StreamParams};

/// Miniaudio backend implementation
pub struct MiniaudioBackend {
    // Implementation will be added when miniaudio bindings are properly set up
}

impl MiniaudioBackend {
    /// Create a new miniaudio backend
    pub fn new() -> Result<Self> {
        // TODO: Implement actual miniaudio context initialization
        Ok(Self {})
    }
}

impl AudioBackend for MiniaudioBackend {
    fn name(&self) -> &'static str {
        "miniaudio"
    }

    fn list_output_devices(&self) -> Result<Vec<DeviceInfo>> {
        // TODO: Implement device enumeration using miniaudio
        // For now, return a placeholder device (default)
        let device_info = DeviceInfo::new(
            DeviceId::new("miniaudio-0".to_string()),
            "Miniaudio Device".to_string(),
            2, // Stereo
            vec![44100, 48000, 88200, 96000],
            true, // only device, so default
        );
        Ok(vec![device_info])
    }

    fn open_output_stream(
        &mut self,
        _device: &DeviceId,
        _params: &StreamParams,
        _callback: Box<dyn AudioCallback>,
    ) -> Result<Box<dyn AudioStream>> {
        // TODO: Implement stream creation using miniaudio
        Err(anyhow::anyhow!("Miniaudio backend not yet implemented"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_miniaudio_backend_creation() {
        let backend = MiniaudioBackend::new();
        assert!(backend.is_ok());
    }

    #[test]
    fn test_miniaudio_backend_name() {
        let backend = MiniaudioBackend::new().unwrap();
        assert_eq!(backend.name(), "miniaudio");
    }

    #[test]
    fn test_miniaudio_device_listing() {
        let backend = MiniaudioBackend::new().unwrap();
        let devices = backend.list_output_devices();
        assert!(devices.is_ok());
        let devices = devices.unwrap();
        assert!(!devices.is_empty());
        assert_eq!(devices[0].id.as_str(), "miniaudio-0");
    }

    #[test]
    fn test_miniaudio_default_device() {
        let backend = MiniaudioBackend::new().unwrap();
        let devices = backend.list_output_devices().unwrap();
        let device = devices.iter().find(|d| d.is_default).or(devices.first()).unwrap();
        assert!(!device.name.is_empty());
        assert_eq!(device.id.as_str(), "miniaudio-0");
        assert!(device.is_default);
    }

    #[test]
    fn test_miniaudio_stream_creation_fails() {
        let mut backend = MiniaudioBackend::new().unwrap();
        let device_id = DeviceId::new("miniaudio-0".to_string());
        let params = StreamParams {
            sample_rate: 48000,
            channels: 2,
            frames_per_buffer: 512,
            low_latency: false,
        };

        // Create a dummy callback
        struct DummyCallback;
        impl AudioCallback for DummyCallback {
            fn render(&mut self, _buffer: &mut [f32], _frames: u32, _sample_rate: u32) {}
        }

        let callback = Box::new(DummyCallback);
        let result = backend.open_output_stream(&device_id, &params, callback);
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e.to_string().contains("not yet implemented"));
        }
    }
}
