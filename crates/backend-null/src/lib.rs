//! Null audio backend for testing and CI
//!
//! Simulates audio timing without requiring actual audio hardware: every opened
//! stream runs a paced driver thread that invokes the engine's audio callback
//! once per buffer duration, exactly like a real sound card. Without that
//! pacing the DSP producer parks forever on the pre-filled ring buffer (see
//! `producer_thread_loop` in `engine-core`), so no deck playhead would advance.

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use anyhow::Result;
use audio_core::{AudioBackend, AudioCallback, AudioStream, DeviceId, DeviceInfo, StreamParams};

/// Null audio backend implementation
#[derive(Debug)]
pub struct NullBackend {
    /// Current buffer size (can be negotiated)
    buffer_size: Arc<AtomicU32>,
}

impl NullBackend {
    /// Create a new null backend
    pub fn new() -> Self {
        Self {
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
        // Return a single virtual device (default)
        Ok(vec![DeviceInfo::new(
            DeviceId::new("null-device"),
            "Null Audio Device".to_string(),
            8,                                // Support up to 8 channels
            vec![44100, 48000, 88200, 96000], // Common sample rates
            true,                             // only device, so default
        )])
    }

    fn open_output_stream(
        &mut self,
        device: &DeviceId,
        params: &StreamParams,
        callback: Box<dyn AudioCallback>,
    ) -> Result<Box<dyn AudioStream>> {
        tracing::info!(
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
        )))
    }
}

/// Null audio stream implementation
struct NullStream {
    /// Stream parameters
    params: StreamParams,
    /// Negotiated buffer size
    negotiated_buffer_size: u32,
    /// Audio callback. Moved into the driver thread while the stream runs.
    callback: Option<Box<dyn AudioCallback>>,
    /// Per-stream running flag (one driver thread per stream).
    running: Arc<AtomicBool>,
    /// Start time for latency calculation
    start_time: Option<Instant>,
    /// Paced driver thread; returns the callback when it exits.
    driver: Option<JoinHandle<Box<dyn AudioCallback>>>,
}

impl NullStream {
    fn new(
        params: StreamParams,
        negotiated_buffer_size: u32,
        callback: Box<dyn AudioCallback>,
    ) -> Self {
        Self {
            params,
            negotiated_buffer_size,
            callback: Some(callback),
            running: Arc::new(AtomicBool::new(false)),
            start_time: None,
            driver: None,
        }
    }
}

impl AudioStream for NullStream {
    fn start(&mut self) -> Result<()> {
        if self.driver.is_some() {
            // Already running.
            return Ok(());
        }

        tracing::info!("Starting null audio stream");
        self.start_time = Some(Instant::now());
        self.running.store(true, Ordering::Relaxed);

        let Some(callback) = self.callback.take() else {
            return Ok(());
        };

        let running = Arc::clone(&self.running);
        let params = self.params.clone();
        let frames = self.negotiated_buffer_size;
        // One callback per buffer duration, like a real sound card. This is what
        // paces the engine's producer thread and advances deck playheads.
        let tick = Duration::from_secs_f64(frames as f64 / params.sample_rate.max(1) as f64);

        self.driver = Some(thread::spawn(move || {
            let mut callback = callback;
            let channels = params.channels.max(1) as usize;
            let mut buffer = vec![0.0; frames as usize * channels];
            while running.load(Ordering::Relaxed) {
                buffer.fill(0.0);
                callback.render(&mut buffer, frames, params.sample_rate);
                thread::sleep(tick);
            }
            callback
        }));

        Ok(())
    }

    fn stop(&mut self) -> Result<()> {
        tracing::info!("Stopping null audio stream");
        self.running.store(false, Ordering::Relaxed);
        self.start_time = None;
        if let Some(driver) = self.driver.take() {
            match driver.join() {
                Ok(callback) => self.callback = Some(callback),
                Err(_) => tracing::error!("Null audio driver thread panicked"),
            }
        }
        Ok(())
    }

    fn actual_buffer_size(&self) -> Option<u32> {
        // Report negotiated size even before start so engine can sync producer to it
        Some(self.negotiated_buffer_size)
    }

    fn actual_sample_rate(&self) -> Option<u32> {
        Some(self.params.sample_rate)
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

impl Drop for NullStream {
    fn drop(&mut self) {
        // A stream dropped while running (e.g. a failed engine start) must not
        // leak its driver thread.
        if self.driver.is_some() {
            let _ = self.stop();
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
        /// Returns the callback plus a handle to its "frames rendered" counter.
        fn new(sample_rate: u32) -> (Self, Arc<AtomicU32>) {
            let frames_processed = Arc::new(AtomicU32::new(0));
            (
                Self {
                    phase: 0.0,
                    sample_rate,
                    frames_processed: Arc::clone(&frames_processed),
                },
                frames_processed,
            )
        }
    }

    impl AudioCallback for TestCallback {
        fn render(&mut self, out: &mut [Sample], frames: u32, _sample_rate: u32) {
            let channels = 2; // Stereo
            let frequency = 440.0; // A4 note
            let amplitude = 0.1;

            for frame in 0..frames {
                let sample =
                    amplitude * (2.0 * std::f32::consts::PI * frequency * self.phase).sin();

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
        let devices = backend.list_output_devices().unwrap();
        let device = devices
            .iter()
            .find(|d| d.is_default)
            .or(devices.first())
            .unwrap();
        assert_eq!(device.name, "Null Audio Device");
        assert!(device.is_default);
    }

    #[test]
    fn test_null_stream_lifecycle() {
        let mut backend = NullBackend::new();
        let device = backend
            .list_output_devices()
            .unwrap()
            .into_iter()
            .next()
            .unwrap();
        let params = StreamParams::new(48000, 2, 512, false);
        let callback = Box::new(TestCallback::new(48000).0);

        let mut stream = backend
            .open_output_stream(&device.id, &params, callback)
            .unwrap();

        // Negotiated buffer size is available before start; latency is not.
        assert_eq!(stream.actual_buffer_size(), Some(512));
        assert!(stream.actual_latency().is_none());

        // Start the stream
        stream.start().unwrap();
        assert_eq!(stream.actual_buffer_size(), Some(512));
        assert!(stream.actual_latency().is_some());

        // Stop the stream — buffer size remains negotiated; latency clears.
        stream.stop().unwrap();
        assert_eq!(stream.actual_buffer_size(), Some(512));
        assert!(stream.actual_latency().is_none());
    }

    #[test]
    fn test_null_stream_audio_processing() {
        let mut backend = NullBackend::new();
        let device = backend
            .list_output_devices()
            .unwrap()
            .into_iter()
            .next()
            .unwrap();
        let params = StreamParams::new(48000, 2, 256, false);
        let (callback, frames_processed) = TestCallback::new(48000);

        let mut stream = backend
            .open_output_stream(&device.id, &params, Box::new(callback))
            .unwrap();

        // Before start the driver is idle, so nothing is rendered.
        assert_eq!(frames_processed.load(Ordering::Relaxed), 0);

        stream.start().unwrap();
        // The paced driver renders on a real-time cadence; 256 frames @ 48 kHz is
        // ~5.3 ms per callback, so a few tens of ms is several callbacks.
        std::thread::sleep(Duration::from_millis(60));
        let rendered = frames_processed.load(Ordering::Relaxed);
        assert!(
            rendered >= 256,
            "driver should have rendered at least one buffer, got {rendered} frames"
        );

        // Stopping must park the driver: no further frames are rendered.
        stream.stop().unwrap();
        let after_stop = frames_processed.load(Ordering::Relaxed);
        std::thread::sleep(Duration::from_millis(30));
        assert_eq!(
            frames_processed.load(Ordering::Relaxed),
            after_stop,
            "driver kept rendering after stop()"
        );
    }

    #[test]
    fn test_buffer_size_negotiation() {
        let mut backend = NullBackend::new();
        backend.set_buffer_size(1024);

        let device = backend
            .list_output_devices()
            .unwrap()
            .into_iter()
            .next()
            .unwrap();
        let params = StreamParams::new(48000, 2, 0, false); // Request 0 to use backend default
        let callback = Box::new(TestCallback::new(48000).0);

        let mut stream = backend
            .open_output_stream(&device.id, &params, callback)
            .unwrap();
        stream.start().unwrap();

        // Should use the backend's default buffer size
        assert_eq!(stream.actual_buffer_size(), Some(1024));
    }
}
