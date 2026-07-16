use crate::routing::{map_buses_to_device_buffer, DeviceStreamPlan};
use crate::transport::TransportEvent;
use anyhow::Result;
use audio_core::{AudioStream, BusId, Sample};
use engine_dsp::DspEngine;
use rtrb::{Consumer, Producer, RingBuffer};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

/// One opened output stream for a `DeviceStreamPlan`, paired with the producer half of its ring.
/// Playback begins after the DSP producer warms up (see `start_device_streams`).
pub(crate) struct DeviceStreamSetup {
    pub plan: DeviceStreamPlan,
    pub stream: Box<dyn AudioStream>,
    pub producer: Producer<Sample>,
    pub callback_count: Arc<AtomicU64>,
    pub callback_frames_atomic: Option<Arc<AtomicU32>>,
    pub sample_rate: u32,
    pub buffer_size: usize,
    pub ring_buffer_capacity: usize,
}

/// Ring buffer: spec §5.2 — preallocate N × frames_per_buffer (N ≥ 8) to tolerate producer jitter.
///
/// Sized in interleaved samples for `channels`, so multi-channel device plans (e.g. a master +
/// cue bus sharing one 4-channel device) reuse the same helper as a plain stereo bus.
pub(crate) fn create_device_ring_buffer(
    buffer_size: u32,
    channels: u16,
) -> (Producer<Sample>, Consumer<Sample>, usize) {
    const RING_BUFFER_MULTIPLIER: usize = 24;
    let samples_per_buffer = buffer_size as usize * channels as usize;
    let ring_buffer_capacity = samples_per_buffer * RING_BUFFER_MULTIPLIER;
    let (mut producer, consumer) = RingBuffer::new(ring_buffer_capacity);

    // Pre-fill with silence so callbacks have data before the producer thread starts (no allocations in callback).
    // Leave 2 buffers free for producer to fill immediately.
    let prefill = ring_buffer_capacity.saturating_sub(2 * samples_per_buffer);
    for _ in 0..prefill {
        let _ = producer.push(0.0);
    }
    log::info!(
        "Ring buffer: channels={}, capacity={}, pre-filled={} samples (spec: N×frames_per_buffer, zero alloc in callback)",
        channels,
        ring_buffer_capacity,
        prefill
    );

    (producer, consumer, ring_buffer_capacity)
}

/// Start every opened device stream, then wait for the pacing (first/master) device to report
/// its callback frame count so the producer's chunk size assumption is confirmed before returning.
pub(crate) fn start_device_streams(
    streams: &mut [Box<dyn AudioStream>],
    expected_buffer_size: u32,
    pacing_callback_frames_atomic: Option<Arc<AtomicU32>>,
) -> Result<()> {
    for stream in streams.iter_mut() {
        stream.start()?;
    }

    if let Some(frames_atomic) = pacing_callback_frames_atomic {
        wait_for_callback_frames(frames_atomic, expected_buffer_size)?;
    }

    Ok(())
}

/// Wait for the audio device to report its callback frame count, then verify it matches config.
pub(crate) fn wait_for_callback_frames(
    frames_atomic: Arc<AtomicU32>,
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

/// Producer thread loop (spec §5.1: writes decoded/resampled audio into per-device ring buffers).
///
/// Production is paced by the pacing device's callback count, not wall clock. The pacing device
/// is `device_producers[0]` — `resolve_device_stream_plans` always sorts a master-hosting plan
/// first when one exists, so index 0 doubles as "the master clock" whenever a master bus is
/// configured, and simply "the only clock" otherwise.
#[allow(clippy::too_many_arguments)]
pub(crate) fn producer_thread_loop(
    dsp_engine: Arc<Mutex<DspEngine>>,
    mut device_producers: Vec<(DeviceStreamPlan, Producer<Sample>)>,
    running: Arc<Mutex<bool>>,
    sample_rate: u32,
    fallback_buffer_size: usize,
    ring_buffer_capacity: usize,
    callback_frames_atomic: Option<Arc<AtomicU32>>,
    callback_count: Arc<AtomicU64>,
    transport_events: Arc<Mutex<Vec<TransportEvent>>>,
) {
    log::info!(
        "Producer thread started ({} device stream(s), fallback_buffer_size={}, ring_capacity={}, sample_rate={})",
        device_producers.len(),
        fallback_buffer_size,
        ring_buffer_capacity,
        sample_rate
    );

    let mut output_buses = HashMap::new();
    output_buses.insert(BusId::new("master"), vec![0.0; fallback_buffer_size * 2]);
    if device_producers
        .iter()
        .any(|(plan, _)| plan.routes.iter().any(|r| r.bus_id.as_str() == "cue"))
    {
        output_buses.insert(BusId::new("cue"), vec![0.0; fallback_buffer_size * 2]);
    }

    let mut device_scratch: Vec<Vec<Sample>> = device_producers
        .iter()
        .map(|(plan, _)| vec![0.0; fallback_buffer_size * plan.channels as usize])
        .collect();

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

        // Never run more than MAX_AHEAD_CHUNKS ahead of the pacing device's callback clock.
        if produced_chunks > device_callbacks.saturating_add(MAX_AHEAD_CHUNKS) {
            thread::sleep(buffer_duration / 4);
            continue;
        }

        let Some((_, pacing_producer)) = device_producers.first() else {
            break;
        };
        let filled = ring_buffer_capacity.saturating_sub(pacing_producer.slots());
        let target_fill = samples_per_chunk * 2;
        if filled >= target_fill || pacing_producer.slots() < samples_per_chunk {
            thread::sleep(buffer_duration / 8);
            continue;
        }

        {
            let mut dsp = dsp_engine.lock().unwrap();
            if let Err(e) = dsp.process(chunk_frames as u32, &mut output_buses) {
                log::error!("DSP processing error: {}", e);
            }
            let deck_events = dsp.drain_transport_events();
            if !deck_events.is_empty() {
                let mut queue = transport_events.lock().unwrap();
                queue.extend(
                    deck_events
                        .into_iter()
                        .map(|(deck_id, event)| TransportEvent::from_deck(deck_id, event)),
                );
            }
        }

        let mut pushed_pacing_chunk = false;
        for (i, (plan, producer)) in device_producers.iter_mut().enumerate() {
            let channels = plan.channels as usize;
            let needed = chunk_frames * channels;
            if device_scratch[i].len() < needed {
                device_scratch[i].resize(needed, 0.0);
            }
            map_buses_to_device_buffer(
                chunk_frames,
                channels,
                &plan.routes,
                &output_buses,
                &mut device_scratch[i][..needed],
            );

            let mut pushed_all = true;
            for &sample in &device_scratch[i][..needed] {
                if producer.push(sample).is_err() {
                    pushed_all = false;
                    break;
                }
            }
            if i == 0 {
                pushed_pacing_chunk = pushed_all;
            }
        }

        if pushed_pacing_chunk {
            produced_chunks += 1;
        }
    }

    log::info!("Producer thread stopped");
}
