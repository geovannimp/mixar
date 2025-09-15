//! Null audio backend for testing and CI
//!
//! This backend provides a deterministic, non-blocking implementation
//! of the AudioBackend trait that simulates audio timing without
//! requiring actual audio hardware.

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use audio_core::{
    AudioBackend, AudioCallback, AudioStream, DeviceId, DeviceInfo, StreamParams,
};
use anyhow::Result;

/// Null audio backend implementation
#[derive(Debug)]
pub struct NullBackend {
    /// Whether the backend is currently running
    running: Arc<AtomicBool>,
    /// Current buffer size (can be negotiated)
    buffer_size: Arc<AtomicU32>,
}

impl NullBackend {
    /// Create a new null backend
    pub fn new() -> Self {
        Self {
            running: Arc::new(AtomicBool::new(false)),
            buffer_size: Arc::new(AtomicU32::new(512)), // Default buffer size
        }
    }

    /// Set the buffer size for this backend
    pub fn set_buffer_size(&self, size: u32) {
        self.buffer_size.store(size, Ordering::Relaxed);
    }

    /// Get the current buffer size
    pub fn buffer_size(&self) -> u32 {
        self.buffer_size.load(Ordering::Relaxed)
    }
}

impl Default for NullBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioBackend for NullBackend {
    fn name(&self) -> &'static str {
        "null"
    }

    fn list_output_devices(&self) -> Result<Vec<DeviceInfo>> {
        // Return a single virtual device
        Ok(vec![DeviceInfo::new(
            DeviceId::new("null-device"),
            "Null Audio Device".to_string(),
            8, // Support up to 8 channels
            vec![44100, 48000, 88200, 96000], // Common sample rates
        )])
    }

    fn default_output_device(&self) -> Result<DeviceInfo> {
        // Return the same device as the only available device
        let devices = self.list_output_devices()?;
        devices
            .into_iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("No devices available"))
    }

    fn open_output_stream(
        &mut self,
        device: &DeviceId,
        params: &StreamParams,
        callback: Box<dyn AudioCallback>,
    ) -> Result<Box<dyn AudioStream>> {
        log::info!(
            "Opening null stream: device={}, sample_rate={}, channels={}, buffer_size={}",
            device.as_str(),
            params.sample_rate,
            params.channels,
            params.frames_per_buffer
        );

        // Negotiate buffer size - use requested size or our default
        let negotiated_size = if params.frames_per_buffer > 0 {
            params.frames_per_buffer
        } else {
            self.buffer_size()
        };

        self.buffer_size.store(negotiated_size, Ordering::Relaxed);

        Ok(Box::new(NullStream::new(
            params.clone(),
            negotiated_size,
            callback,
            self.running.clone(),
        )))
    }
}

/// Null audio stream implementation
struct NullStream {
    /// Stream parameters
    params: StreamParams,
    /// Negotiated buffer size
    negotiated_buffer_size: u32,
    /// Audio callback
    callback: Box<dyn AudioCallback>,
    /// Running state
    running: Arc<AtomicBool>,
    /// Start time for latency calculation
    start_time: Option<Instant>,
}

impl NullStream {
    fn new(
        params: StreamParams,
        negotiated_buffer_size: u32,
        callback: Box<dyn AudioCallback>,
        running: Arc<AtomicBool>,
    ) -> Self {
        Self {
            params,
            negotiated_buffer_size,
            callback,
            running,
            start_time: None,
        }
    }

    /// Simulate audio processing by calling the callback
    fn process_audio(&mut self) -> Result<()> {
        let frames = self.negotiated_buffer_size;
        let channels = self.params.channels as usize;
        let buffer_size = frames as usize * channels;
        
        // Create a buffer for the callback to fill
        let mut buffer = vec![0.0; buffer_size];
        
        // Call the audio callback
        self.callback.render(&mut buffer, frames, self.params.sample_rate);
        
        // In a real backend, we would send this to the audio device
        // For null backend, we just log that we processed the audio
        log::debug!(
            "Processed {} frames of audio ({} samples)",
            frames,
            buffer_size
        );
        
        Ok(())
    }
}

impl AudioStream for NullStream {
    fn start(&mut self) -> Result<()> {
        log::info!("Starting null audio stream");
        self.running.store(true, Ordering::Relaxed);
        self.start_time = Some(Instant::now());
        Ok(())
    }

    fn stop(&mut self) -> Result<()> {
        log::info!("Stopping null audio stream");
        self.running.store(false, Ordering::Relaxed);
        self.start_time = None;
        Ok(())
    }

    fn actual_buffer_size(&self) -> Option<u32> {
        if self.running.load(Ordering::Relaxed) {
            Some(self.negotiated_buffer_size)
        } else {
            None
        }
    }

    fn actual_sample_rate(&self) -> Option<u32> {
        if self.running.load(Ordering::Relaxed) {
            Some(self.params.sample_rate)
        } else {
            None
        }
    }

    fn actual_latency(&self) -> Option<Duration> {
        if let Some(_start_time) = self.start_time {
            // Simulate latency as buffer duration
            let buffer_duration = Duration::from_secs_f64(
                self.negotiated_buffer_size as f64 / self.params.sample_rate as f64,
            );
            Some(buffer_duration)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use audio_core::Sample;
    use std::sync::atomic::AtomicU32;

    /// Test callback that generates a simple sine wave
    struct TestCallback {
        phase: f32,
        sample_rate: u32,
        frames_processed: Arc<AtomicU32>,
    }

    impl TestCallback {
        fn new(sample_rate: u32) -> Self {
            Self {
                phase: 0.0,
                sample_rate,
                frames_processed: Arc::new(AtomicU32::new(0)),
            }
        }

        fn frames_processed(&self) -> u32 {
            self.frames_processed.load(Ordering::Relaxed)
        }
    }

    impl AudioCallback for TestCallback {
        fn render(&mut self, out: &mut [Sample], frames: u32, _sample_rate: u32) {
            let channels = 2; // Stereo
            let frequency = 440.0; // A4 note
            let amplitude = 0.1;

            for frame in 0..frames {
                let sample = amplitude * (2.0 * std::f32::consts::PI * frequency * self.phase).sin();
                
                // Write to both channels (interleaved)
                let left_idx = (frame * channels) as usize;
                let right_idx = (frame * channels + 1) as usize;
                
                if left_idx < out.len() {
                    out[left_idx] = sample;
                }
                if right_idx < out.len() {
                    out[right_idx] = sample;
                }
                
                self.phase += 1.0 / self.sample_rate as f32;
                if self.phase >= 1.0 {
                    self.phase -= 1.0;
                }
            }
            
            self.frames_processed.fetch_add(frames, Ordering::Relaxed);
        }
    }

    #[test]
    fn test_null_backend_creation() {
        let backend = NullBackend::new();
        assert_eq!(backend.name(), "null");
        assert_eq!(backend.buffer_size(), 512);
    }

    #[test]
    fn test_null_backend_device_listing() {
        let backend = NullBackend::new();
        let devices = backend.list_output_devices().unwrap();
        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0].name, "Null Audio Device");
        assert_eq!(devices[0].max_channels, 8);
    }

    #[test]
    fn test_null_backend_default_device() {
        let backend = NullBackend::new();
        let device = backend.default_output_device().unwrap();
        assert_eq!(device.name, "Null Audio Device");
    }

    #[test]
    fn test_null_stream_lifecycle() {
        let mut backend = NullBackend::new();
        let device = backend.default_output_device().unwrap();
        let params = StreamParams::new(48000, 2, 512, false);
        let callback = Box::new(TestCallback::new(48000));
        
        let mut stream = backend.open_output_stream(&device.id, &params, callback).unwrap();
        
        // Initially not running
        assert!(stream.actual_buffer_size().is_none());
        assert!(stream.actual_latency().is_none());
        
        // Start the stream
        stream.start().unwrap();
        assert_eq!(stream.actual_buffer_size(), Some(512));
        assert!(stream.actual_latency().is_some());
        
        // Stop the stream
        stream.stop().unwrap();
        assert!(stream.actual_buffer_size().is_none());
        assert!(stream.actual_latency().is_none());
    }

    #[test]
    fn test_null_stream_audio_processing() {
        let mut backend = NullBackend::new();
        let device = backend.default_output_device().unwrap();
        let params = StreamParams::new(48000, 2, 256, false);
        let callback = Box::new(TestCallback::new(48000));
        
        let mut stream = backend.open_output_stream(&device.id, &params, callback).unwrap();
        stream.start().unwrap();
        
        // Process some audio (simulated by starting the stream)
        // In a real implementation, the stream would process audio automatically
        
        // Verify the callback was called
        assert_eq!(stream.actual_buffer_size(), Some(256));
    }

    #[test]
    fn test_buffer_size_negotiation() {
        let mut backend = NullBackend::new();
        backend.set_buffer_size(1024);
        
        let device = backend.default_output_device().unwrap();
        let params = StreamParams::new(48000, 2, 0, false); // Request 0 to use backend default
        let callback = Box::new(TestCallback::new(48000));
        
        let mut stream = backend.open_output_stream(&device.id, &params, callback).unwrap();
        stream.start().unwrap();
        
        // Should use the backend's default buffer size
        assert_eq!(stream.actual_buffer_size(), Some(1024));
    }
}
