//! CPAL backend implementation
//!
//! This backend uses the CPAL library for cross-platform audio I/O.
//! CPAL is a mature and actively maintained audio library for Rust.

use anyhow::Result;
use audio_core::{AudioBackend, AudioCallback, AudioStream, DeviceId, DeviceInfo, StreamParams};
use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    BuildStreamError, Host, Stream, StreamConfig,
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
        let output_devices = self.host.output_devices()?;
        for device in output_devices {
            if let Ok(id) = device.id() {
                if id.to_string() == suffix {
                    return Ok(device);
                }
            }
        }
        Err(anyhow::anyhow!("Device not found: {}", id_str))
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
            let is_default = default_id.as_ref().map_or(false, |d| d.as_str() == id.as_str());

            // Get supported sample rates (cpal 0.17: SampleRate is u32)
            let mut sample_rates = vec![44100, 48000];
            if let Ok(configs) = device.supported_output_configs() {
                for config in configs {
                    sample_rates.push(config.min_sample_rate());
                    sample_rates.push(config.max_sample_rate());
                }
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
        let cpal_device = self.find_device_by_id(device)?;

        // Query supported configurations and find the best match (cpal 0.17: sample rates are u32)
        let desired_sample_rate = params.sample_rate;
        let supported_configs: Vec<_> = cpal_device.supported_output_configs()?.collect();

        // Log all supported configurations for debugging
        log::info!("Available device configurations:");
        for (i, config) in supported_configs.iter().enumerate() {
            log::info!(
                "  {}: {} channels, {} Hz - {} Hz",
                i,
                config.channels(),
                config.min_sample_rate(),
                config.max_sample_rate()
            );
        }

        // Find a supported configuration that matches our desired sample rate
        // Prefer stereo (2 channels) if available, otherwise use the first available
        let matching_configs: Vec<_> = supported_configs
            .iter()
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

        let actual_sample_rate = supported_config.sample_rate();
        log::info!(
            "Using CPAL config: {} Hz, {} channels, buffer size: {}",
            actual_sample_rate,
            supported_config.channels(),
            params.frames_per_buffer
        );

        let channels = supported_config.channels();
        let frames_per_buffer = params.frames_per_buffer;

        let callback_arc = Arc::new(Mutex::new(callback));
        let last_callback_frames = Arc::new(AtomicU32::new(0));

        // Prefer fixed buffer size for low latency; fall back to Default if not supported (e.g. JACK).
        let build_stream = |buffer_size: cpal::BufferSize| {
            let config = StreamConfig {
                channels: supported_config.channels(),
                sample_rate: actual_sample_rate,
                buffer_size,
            };
            let arc = Arc::clone(&callback_arc);
            let frames = Arc::clone(&last_callback_frames);
            cpal_device.build_output_stream(
                &config,
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

        let stream = match build_stream(cpal::BufferSize::Fixed(params.frames_per_buffer)) {
            Ok(s) => s,
            Err(BuildStreamError::StreamConfigNotSupported) => {
                log::info!(
                    "Fixed buffer size {} not supported, using host default for low latency",
                    params.frames_per_buffer
                );
                build_stream(cpal::BufferSize::Default)?
            }
            Err(e) => return Err(e.into()),
        };

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
        let devices = backend.list_output_devices().unwrap();
        let default = devices.iter().find(|d| d.is_default).or(devices.first());
        if let Some(d) = default {
            assert!(!d.name.is_empty());
        }
    }
}
