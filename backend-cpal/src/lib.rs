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
use std::sync::atomic::{AtomicU32, Ordering};
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

    /// Get device ID from device name
    fn get_device_id_from_name(&self, device_name: &str) -> DeviceId {
        DeviceId::new(format!("cpal:{}", device_name))
    }

    /// Get device name from device ID
    fn get_device_name_from_id(&self, device_id: &DeviceId) -> Result<String> {
        let id_str = device_id.as_str();
        if let Some(name) = id_str.strip_prefix("cpal:") {
            Ok(name.to_string())
        } else {
            Err(anyhow::anyhow!("Invalid CPAL device ID format: {}", id_str))
        }
    }

    /// Find device by name
    fn find_device_by_name(&self, target_name: &str) -> Result<cpal::Device> {
        let output_devices = self.host.output_devices()?;
        for device in output_devices {
            if let Ok(name) = device.name() {
                if name == target_name {
                    return Ok(device);
                }
            }
        }
        Err(anyhow::anyhow!("Device not found: {}", target_name))
    }
}

impl AudioBackend for CpalBackend {
    fn name(&self) -> &'static str {
        "cpal"
    }

    fn list_output_devices(&self) -> Result<Vec<DeviceInfo>> {
        let mut devices = Vec::new();

        let output_devices = self.host.output_devices()?;
        for device in output_devices {
            let device_name = device
                .name()
                .unwrap_or_else(|_| "Unknown Device".to_string());

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
                self.get_device_id_from_name(&device_name),
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
            self.get_device_id_from_name(&device_name),
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
        let device_name = self.get_device_name_from_id(device)?;
        let cpal_device = self.find_device_by_name(&device_name)?;

        // Query supported configurations and find the best match
        let supported_configs = cpal_device.supported_output_configs()?;
        let desired_sample_rate = SampleRate(params.sample_rate);

        // Log all supported configurations for debugging
        log::info!("Available device configurations:");
        for (i, config) in supported_configs.enumerate() {
            log::info!(
                "  {}: {} channels, {} Hz - {} Hz",
                i,
                config.channels(),
                config.min_sample_rate().0,
                config.max_sample_rate().0
            );
        }

        // Find a supported configuration that matches our desired sample rate
        // Prefer stereo (2 channels) if available, otherwise use the first available
        let supported_configs = cpal_device.supported_output_configs()?;
        let matching_configs: Vec<_> = supported_configs
            .filter(|config| {
                config.min_sample_rate() <= desired_sample_rate
                    && config.max_sample_rate() >= desired_sample_rate
            })
            .collect();

        let supported_config = matching_configs
            .iter()
            .find(|config| config.channels() == 2) // Prefer stereo
            .or_else(|| matching_configs.first()) // Fallback to first available
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "No supported config found for sample rate {}",
                    params.sample_rate
                )
            })?
            .with_sample_rate(desired_sample_rate);

        log::info!(
            "Using CPAL config: {} Hz, {} channels, buffer size: {}",
            supported_config.sample_rate().0,
            supported_config.channels(),
            params.frames_per_buffer
        );

        let config = StreamConfig {
            channels: supported_config.channels(),
            sample_rate: supported_config.sample_rate(),
            buffer_size: cpal::BufferSize::Fixed(params.frames_per_buffer),
        };

        // Clone the parameters to move into the closure
        let channels = supported_config.channels();
        let actual_sample_rate = supported_config.sample_rate().0;
        let frames_per_buffer = params.frames_per_buffer;

        let callback_arc = Arc::new(Mutex::new(callback));
        // Record actual frames per callback (driver may use different size than requested)
        let last_callback_frames = Arc::new(AtomicU32::new(0));

        let last_callback_frames_clone = last_callback_frames.clone();
        let stream = cpal_device.build_output_stream(
            &config,
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                let frames = data.len() / channels as usize;
                last_callback_frames_clone.store(frames as u32, Ordering::Relaxed);
                if let Ok(mut callback) = callback_arc.lock() {
                    callback.render(data, frames as u32, actual_sample_rate);
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
        let from_callback = self.last_callback_frames.load(Ordering::Relaxed);
        Some(if from_callback > 0 {
            from_callback
        } else {
            self.frames_per_buffer
        })
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
        let device = backend.default_output_device();
        // This might fail if no devices are available, which is ok for testing
        if device.is_ok() {
            assert!(!device.unwrap().name.is_empty());
        }
    }
}
