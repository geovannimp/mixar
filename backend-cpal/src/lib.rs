//! CPAL backend implementation
//!
//! This backend uses the CPAL library for cross-platform audio I/O.
//! CPAL is a mature and actively maintained audio library for Rust.

use anyhow::Result;
use audio_core::{AudioBackend, AudioCallback, AudioStream, DeviceId, DeviceInfo, StreamParams};
use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    Host, SampleRate, Stream, StreamConfig,
};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// CPAL backend implementation
pub struct CpalBackend {
    host: Host,
}

impl CpalBackend {
    /// Create a new CPAL backend
    pub fn new() -> Result<Self> {
        let host = cpal::default_host();
        Ok(Self { host })
    }
}

impl AudioBackend for CpalBackend {
    fn name(&self) -> &'static str {
        "cpal"
    }

    fn list_output_devices(&self) -> Result<Vec<DeviceInfo>> {
        let mut devices = Vec::new();

        let output_devices = self.host.output_devices()?;
        for (index, device) in output_devices.enumerate() {
            let device_name = device
                .name()
                .unwrap_or_else(|_| format!("Device {}", index));

            // Get supported sample rates
            let mut sample_rates = vec![44100, 48000];
            if let Ok(configs) = device.supported_output_configs() {
                for config in configs {
                    sample_rates.push(config.min_sample_rate().0);
                    sample_rates.push(config.max_sample_rate().0);
                }
            }
            sample_rates.sort();
            sample_rates.dedup();

            let device_info = DeviceInfo::new(
                DeviceId::new(format!("cpal-{}", index)),
                device_name,
                2, // Default to stereo
                sample_rates,
            );
            devices.push(device_info);
        }

        Ok(devices)
    }

    fn default_output_device(&self) -> Result<DeviceInfo> {
        let default_device = self
            .host
            .default_output_device()
            .ok_or_else(|| anyhow::anyhow!("No default output device available"))?;

        let device_name = default_device
            .name()
            .unwrap_or_else(|_| "Default Device".to_string());

        let device_info = DeviceInfo::new(
            DeviceId::new("cpal-0".to_string()),
            device_name,
            2, // Default to stereo
            vec![44100, 48000, 88200, 96000],
        );

        Ok(device_info)
    }

    fn open_output_stream(
        &mut self,
        device: &DeviceId,
        params: &StreamParams,
        callback: Box<dyn AudioCallback>,
    ) -> Result<Box<dyn AudioStream>> {
        let device_index = device
            .as_str()
            .strip_prefix("cpal-")
            .and_then(|s| s.parse::<usize>().ok())
            .ok_or_else(|| anyhow::anyhow!("Invalid device ID: {}", device.as_str()))?;

        let output_devices: Vec<_> = self.host.output_devices()?.collect();
        let cpal_device = output_devices
            .get(device_index)
            .ok_or_else(|| anyhow::anyhow!("Device not found: {}", device_index))?;

        let config = StreamConfig {
            channels: params.channels as u16,
            sample_rate: SampleRate(params.sample_rate),
            buffer_size: cpal::BufferSize::Fixed(params.frames_per_buffer),
        };

        // Clone the parameters to move into the closure
        let channels = params.channels;
        let sample_rate = params.sample_rate;
        let frames_per_buffer = params.frames_per_buffer;

        let callback_arc = Arc::new(Mutex::new(callback));

        let stream = cpal_device.build_output_stream(
            &config,
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                if let Ok(mut callback) = callback_arc.lock() {
                    let frames = data.len() / channels as usize;
                    callback.render(data, frames as u32, sample_rate);
                }
            },
            |err| {
                eprintln!("Audio stream error: {}", err);
            },
            None,
        )?;

        Ok(Box::new(CpalStream {
            stream,
            frames_per_buffer,
            sample_rate,
        }))
    }
}

/// CPAL stream implementation
struct CpalStream {
    stream: Stream,
    frames_per_buffer: u32,
    sample_rate: u32,
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
        Some(self.frames_per_buffer)
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
        // Note: The actual number of devices depends on the system
    }

    #[test]
    fn test_cpal_default_device() {
        let backend = CpalBackend::new().unwrap();
        let device = backend.default_output_device();
        // This might fail if no devices are available, which is ok for testing
        if device.is_ok() {
            assert!(!device.unwrap().name.is_empty());
        }
    }
}
