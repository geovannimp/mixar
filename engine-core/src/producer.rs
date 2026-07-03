use anyhow::Result;
use audio_core::{AudioStream, BusId, Sample};
use engine_dsp::DspEngine;
use rtrb::{Consumer, Producer, RingBuffer};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

/// Opened master output stream; playback begins after the DSP producer warms up.
pub(crate) struct MasterStreamSetup {
    pub stream: Box<dyn AudioStream>,
    pub callback_count: Arc<AtomicU64>,
    pub callback_frames_atomic: Option<Arc<std::sync::atomic::AtomicU32>>,
    pub sample_rate: u32,
    pub buffer_size: usize,
}

impl MasterStreamSetup {
    pub(crate) fn start_playback(
        mut self,
        expected_buffer_size: u32,
    ) -> Result<Box<dyn AudioStream>> {
        self.stream.start()?;

        if let Some(frames_atomic) = self.callback_frames_atomic {
            wait_for_callback_frames(frames_atomic, expected_buffer_size)?;
        }

        Ok(self.stream)
    }
}

/// Ring buffer: spec §5.2 — preallocate N × frames_per_buffer (N ≥ 8) to tolerate producer jitter.
pub(crate) fn create_ring_buffer(
    buffer_size: u32,
) -> (Producer<Sample>, Consumer<Sample>, usize) {
    const RING_BUFFER_MULTIPLIER: usize = 24;
    let stereo_samples_per_buffer = buffer_size as usize * 2;
    let ring_buffer_capacity = stereo_samples_per_buffer * RING_BUFFER_MULTIPLIER;
    let (mut producer, consumer) = RingBuffer::new(ring_buffer_capacity);

    // Pre-fill with silence so callbacks have data before the producer thread starts (no allocations in callback).
    // Leave 2 buffers free for producer to fill immediately.
    let prefill = ring_buffer_capacity.saturating_sub(2 * stereo_samples_per_buffer);
    for _ in 0..prefill {
        let _ = producer.push(0.0);
    }
    log::info!(
        "Ring buffer: capacity={}, pre-filled={} samples (spec: N×frames_per_buffer, zero alloc in callback)",
        ring_buffer_capacity,
        prefill
    );

    (producer, consumer, ring_buffer_capacity)
}

/// Wait for the audio device to report its callback frame count, then verify it matches config.
pub(crate) fn wait_for_callback_frames(
    frames_atomic: Arc<std::sync::atomic::AtomicU32>,
    expected_frames: u32,
) -> Result<()> {
    const TIMEOUT: Duration = Duration::from_secs(2);
    let deadline = Instant::now() + TIMEOUT;

    while Instant::now() < deadline {
        let frames = frames_atomic.load(Ordering::Relaxed);
        if frames > 0 {
            if frames != expected_frames {
                return Err(anyhow::anyhow!(
                    "Device callback size is {} frames but {} frames were configured",
                    frames,
                    expected_frames
                ));
            }
            log::info!("Device callback size verified: {} frames", frames);
            return Ok(());
        }
        thread::sleep(Duration::from_millis(1));
    }

    Err(anyhow::anyhow!(
        "Timed out waiting for audio device callback (expected {} frames)",
        expected_frames
    ))
}

/// Producer thread loop (spec §5.1: writes decoded/resampled audio into ring buffer).
/// Production is paced by the audio device callback count, not wall clock.
#[allow(clippy::too_many_arguments)]
pub(crate) fn producer_thread_loop(
    dsp_engine: Arc<Mutex<DspEngine>>,
    mut producer: Producer<Sample>,
    running: Arc<Mutex<bool>>,
    sample_rate: u32,
    fallback_buffer_size: usize,
    ring_buffer_capacity: usize,
    callback_frames_atomic: Option<Arc<std::sync::atomic::AtomicU32>>,
    callback_count: Arc<AtomicU64>,
) {
    log::info!(
        "Producer thread started (fallback_buffer_size={}, ring_capacity={}, sample_rate={})",
        fallback_buffer_size,
        ring_buffer_capacity,
        sample_rate
    );

    let master_bus_id = BusId::new("master");
    let mut output_buses = HashMap::new();
    output_buses.insert(master_bus_id.clone(), vec![0.0; fallback_buffer_size * 2]);

    const MAX_AHEAD_CHUNKS: u64 = 2;
    let mut produced_chunks: u64 = 0;

    while *running.lock().unwrap() {
        let chunk_frames = callback_frames_atomic
            .as_ref()
            .and_then(|a| {
                let v = a.load(Ordering::Relaxed);
                if v > 0 {
                    Some(v as usize)
                } else {
                    None
                }
            })
            .unwrap_or(fallback_buffer_size);
        let samples_per_chunk = chunk_frames * 2;

        let buffer_duration = Duration::from_secs_f64(chunk_frames as f64 / sample_rate as f64);

        let device_callbacks = callback_count.load(Ordering::Relaxed);

        // Never run more than MAX_AHEAD_CHUNKS ahead of the device callback clock.
        if produced_chunks > device_callbacks.saturating_add(MAX_AHEAD_CHUNKS) {
            thread::sleep(buffer_duration / 4);
            continue;
        }

        let filled = ring_buffer_capacity.saturating_sub(producer.slots());
        let target_fill = samples_per_chunk * 2;
        if filled >= target_fill || producer.slots() < samples_per_chunk {
            thread::sleep(buffer_duration / 8);
            continue;
        }

        {
            let mut dsp = dsp_engine.lock().unwrap();
            if let Err(e) = dsp.process(chunk_frames as u32, &mut output_buses) {
                log::error!("DSP processing error: {}", e);
            }
        }

        let mut pushed_chunk = false;
        if let Some(master_bus) = output_buses.get(&master_bus_id) {
            pushed_chunk = true;
            for &sample in master_bus.iter().take(samples_per_chunk) {
                if producer.push(sample).is_err() {
                    pushed_chunk = false;
                    break;
                }
            }
        }

        if pushed_chunk {
            produced_chunks += 1;
        }
    }

    log::info!("Producer thread stopped");
}
