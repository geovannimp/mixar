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
    tracing::info!(
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
            tracing::info!("Device callback size verified: {} frames", frames);
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
    tracing::info!(
        "Producer thread started ({} device stream(s), fallback_buffer_size={}, ring_capacity={}, sample_rate={})",
        device_producers.len(),
        fallback_buffer_size,
        ring_buffer_capacity,
        sample_rate
    );

    // The engine already verified at stream open that every device's actual
    // buffer size matches config.buffer_size, and start_device_streams waits for
    // the pacing callback to report that same size. Treat fallback_buffer_size
    // as the contractual upper bound for callback frames and pre-size all
    // producer scratch/output buffers to it so the steady-state loop never allocates.
    let max_expected_frames = fallback_buffer_size;

    let mut output_buses = HashMap::new();
    output_buses.insert(BusId::new("master"), vec![0.0; max_expected_frames * 2]);
    if device_producers
        .iter()
        .any(|(plan, _)| plan.routes.iter().any(|r| r.bus_id.as_str() == "cue"))
    {
        output_buses.insert(BusId::new("cue"), vec![0.0; max_expected_frames * 2]);
    }

    let mut device_scratch: Vec<Vec<Sample>> = device_producers
        .iter()
        .map(|(plan, _)| vec![0.0; max_expected_frames * plan.channels as usize])
        .collect();

    const MAX_AHEAD_CHUNKS: u64 = 2;
    let mut produced_chunks: u64 = 0;
    let mut oversized_warned = false;
    let pacing_channels = device_producers
        .first()
        .map(|(plan, _)| plan.channels as usize)
        .unwrap_or(2);

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

        // Hard error: callback frames must never exceed the pre-sized bound. On
        // violation, log once for this producer thread, skip the chunk, and let the
        // ring underrun into silence rather than allocating or passing an oversized
        // frame count into DSP. The flag is intentionally never re-armed.
        if chunk_frames > max_expected_frames {
            if !oversized_warned {
                tracing::error!(
                    "Producer callback size {} frames exceeds pre-sized max {} frames; skipping chunk. Driver buffer changed after stream open?",
                    chunk_frames,
                    max_expected_frames
                );
                oversized_warned = true;
            }
            thread::sleep(
                Duration::from_secs_f64(max_expected_frames as f64 / sample_rate as f64) / 4,
            );
            continue;
        }

        let samples_per_chunk = chunk_frames * pacing_channels;

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

        // Do not start a chunk unless every device ring can accept a full chunk.
        // This prevents partial writes that would desync devices.
        let all_have_room = device_producers.iter().all(|(plan, producer)| {
            let needed = chunk_frames * plan.channels as usize;
            producer.slots() >= needed
        });
        if !all_have_room {
            thread::sleep(buffer_duration / 8);
            continue;
        }

        {
            let mut dsp = dsp_engine.lock().unwrap();
            if let Err(e) = dsp.process(chunk_frames as u32, &mut output_buses) {
                tracing::error!("DSP processing error: {}", e);
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
            map_buses_to_device_buffer(
                chunk_frames,
                channels,
                &plan.routes,
                &output_buses,
                &mut device_scratch[i][..needed],
            );

            let pushed_all = push_ring_samples(producer, &device_scratch[i][..needed]);
            if i == 0 {
                pushed_pacing_chunk = pushed_all;
            }
        }

        if pushed_pacing_chunk {
            produced_chunks += 1;
        }
    }

    tracing::info!("Producer thread stopped");
}

/// Bulk-write interleaved scratch into the ring. Returns `true` iff every sample was committed
/// (same semantics as the former per-sample `push` loop: partial/full-ring → `false`).
fn push_ring_samples(producer: &mut Producer<Sample>, samples: &[Sample]) -> bool {
    if samples.is_empty() {
        return true;
    }

    let n = producer.slots().min(samples.len());
    if n == 0 {
        return false;
    }
    let Ok(mut chunk) = producer.write_chunk(n) else {
        return false;
    };
    let (first, second) = chunk.as_mut_slices();
    let first_take = first.len().min(n);
    let second_take = n - first_take;
    first[..first_take].copy_from_slice(&samples[..first_take]);
    second[..second_take].copy_from_slice(&samples[first_take..n]);
    chunk.commit_all();
    n == samples.len()
}

#[cfg(test)]
mod tests {
    use super::*;
    // Arc, Mutex, thread, Duration and the atomics all resolve via `use super::*`.

    #[test]
    fn push_ring_samples_writes_full_chunk() {
        let (mut producer, mut consumer) = RingBuffer::new(16);
        let samples = [0.1, 0.2, 0.3, 0.4];

        assert!(push_ring_samples(&mut producer, &samples));
        assert_eq!(consumer.slots(), 4);

        let chunk = consumer.read_chunk(4).unwrap();
        let (first, second) = chunk.as_slices();
        let mut got = Vec::with_capacity(4);
        got.extend_from_slice(first);
        got.extend_from_slice(second);
        chunk.commit_all();
        assert_eq!(got.as_slice(), &samples);
    }

    #[test]
    fn push_ring_samples_partial_when_ring_nearly_full() {
        let (mut producer, mut consumer) = RingBuffer::new(4);
        assert!(push_ring_samples(&mut producer, &[1.0, 2.0, 3.0]));

        let samples = [4.0, 5.0, 6.0];
        assert!(!push_ring_samples(&mut producer, &samples));
        assert_eq!(producer.slots(), 0);

        let chunk = consumer.read_chunk(4).unwrap();
        let (first, second) = chunk.as_slices();
        let mut got = Vec::with_capacity(4);
        got.extend_from_slice(first);
        got.extend_from_slice(second);
        chunk.commit_all();
        assert_eq!(got.as_slice(), &[1.0, 2.0, 3.0, 4.0]);
    }

    #[test]
    fn push_ring_samples_returns_false_when_ring_full() {
        let (mut producer, _consumer) = RingBuffer::new(2);
        assert!(push_ring_samples(&mut producer, &[1.0, 2.0]));
        assert!(!push_ring_samples(&mut producer, &[3.0, 4.0]));
    }

    #[test]
    fn two_device_starved_nonpacing_ring_stays_aligned() {
        use crate::routing::BusRoute;
        use audio_core::DeviceId;
        use engine_dsp::SamplerStripRoute;

        const BUFFER_FRAMES: u32 = 64; // dasp_graph::Buffer::LEN
        const CHANNELS: u16 = 2;
        const CHUNK_SAMPLES: usize = BUFFER_FRAMES as usize * CHANNELS as usize;
        const RING_CAPACITY: usize = 512;

        let pacing_plan = DeviceStreamPlan {
            device: DeviceId::new("pacing"),
            channels: CHANNELS,
            routes: vec![BusRoute {
                bus_id: BusId::new("master"),
                left: 0,
                right: Some(1),
            }],
        };
        let nonpacing_plan = DeviceStreamPlan {
            device: DeviceId::new("nonpacing"),
            channels: CHANNELS,
            routes: vec![BusRoute {
                bus_id: BusId::new("cue"),
                left: 0,
                right: Some(1),
            }],
        };

        let (pacing_prod, pacing_cons) = RingBuffer::<Sample>::new(RING_CAPACITY);
        let (mut nonpacing_prod, mut nonpacing_cons) = RingBuffer::<Sample>::new(RING_CAPACITY);

        // Starve the non-pacing ring: fewer free slots than one chunk.
        let prefill = RING_CAPACITY - (CHUNK_SAMPLES - 1);
        assert!(push_ring_samples(&mut nonpacing_prod, &vec![0.0; prefill],));

        let dsp_engine = Arc::new(Mutex::new(DspEngine::new(
            48_000,
            BUFFER_FRAMES,
            0,
            "medium",
            SamplerStripRoute::BeforeStrip,
        )));

        let running = Arc::new(Mutex::new(true));
        let running_thread = Arc::clone(&running);
        let handle = thread::spawn(move || {
            producer_thread_loop(
                dsp_engine,
                vec![(pacing_plan, pacing_prod), (nonpacing_plan, nonpacing_prod)],
                running_thread,
                48_000,
                BUFFER_FRAMES as usize,
                RING_CAPACITY,
                Some(Arc::new(AtomicU32::new(BUFFER_FRAMES))),
                Arc::new(AtomicU64::new(1000)),
                Arc::new(Mutex::new(Vec::new())),
            );
        });

        // Give the producer a chance to run while the non-pacing ring is starved.
        thread::sleep(Duration::from_millis(10));
        assert_eq!(
            pacing_cons.slots(),
            0,
            "producer must not write when a non-pacing ring lacks room for a full chunk"
        );

        // Free two full chunks on the non-pacing ring.
        let chunk = nonpacing_cons.read_chunk(2 * CHUNK_SAMPLES).unwrap();
        chunk.commit_all();

        // Wait for the producer to write at least one full chunk.
        let deadline = Instant::now() + Duration::from_millis(500);
        while pacing_cons.slots() < CHUNK_SAMPLES && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(1));
        }

        *running.lock().unwrap() = false;
        handle.join().unwrap();

        let nonpacing_base = prefill - (2 * CHUNK_SAMPLES);
        let nonpacing_produced = nonpacing_cons.slots() - nonpacing_base;

        assert!(
            pacing_cons.slots() > 0,
            "pacing ring should have produced at least one chunk"
        );
        assert_eq!(
            pacing_cons.slots() % CHUNK_SAMPLES,
            0,
            "pacing ring must only contain whole chunks"
        );
        assert_eq!(
            nonpacing_produced % CHUNK_SAMPLES,
            0,
            "non-pacing ring must not contain a partial commit"
        );
        assert_eq!(
            pacing_cons.slots(),
            nonpacing_produced,
            "both devices must produce the same number of new samples"
        );
    }

    #[test]
    fn producer_skips_oversized_callback_without_resizing() {
        use crate::routing::BusRoute;
        use audio_core::{BusId, DeviceId};
        use engine_dsp::SamplerStripRoute;

        let fallback_buffer_size = 64;
        let sample_rate = 48000;
        let channels = 2;

        let plan = DeviceStreamPlan {
            device: DeviceId::new("null-device"),
            channels,
            routes: vec![BusRoute {
                bus_id: BusId::new("master"),
                left: 0,
                right: Some(1),
            }],
        };

        let (producer, mut consumer, ring_buffer_capacity) =
            create_device_ring_buffer(fallback_buffer_size as u32, channels);
        // Drain the pre-filled silence: otherwise the fill-level gate blocks production
        // and the oversized assertion passes even without the guard.
        let prefill = consumer.slots();
        consumer.read_chunk(prefill).unwrap().commit_all();

        let dsp_engine = Arc::new(Mutex::new(DspEngine::new(
            sample_rate,
            fallback_buffer_size as u32,
            2,
            "medium",
            SamplerStripRoute::BeforeStrip,
        )));

        let running = Arc::new(Mutex::new(true));
        let transport_events: Arc<Mutex<Vec<TransportEvent>>> = Arc::new(Mutex::new(Vec::new()));
        // Large callback count so the pacing gate never blocks production.
        let callback_count = Arc::new(AtomicU64::new(u64::MAX / 4));
        let callback_frames = Arc::new(AtomicU32::new(fallback_buffer_size as u32));

        let running_clone = Arc::clone(&running);
        let callback_frames_clone = Arc::clone(&callback_frames);
        let thread_handle = thread::spawn(move || {
            producer_thread_loop(
                dsp_engine,
                vec![(plan, producer)],
                running_clone,
                sample_rate,
                fallback_buffer_size,
                ring_buffer_capacity,
                Some(callback_frames_clone),
                callback_count,
                transport_events,
            );
        });

        // Precondition: the producer really runs at the normal size, otherwise the
        // oversized assertion below could pass vacuously.
        let deadline = std::time::Instant::now() + Duration::from_secs(2);
        while consumer.slots() == 0 {
            assert!(
                std::time::Instant::now() < deadline,
                "producer never produced a normal-size chunk"
            );
            thread::sleep(Duration::from_millis(1));
        }

        // Switch to an oversized callback, give the producer time to observe it and
        // emit any chunk already in flight, then drain that chunk.
        callback_frames.store(fallback_buffer_size as u32 * 2, Ordering::Relaxed);
        thread::sleep(Duration::from_millis(100));
        let pending = consumer.slots();
        if pending > 0 {
            consumer.read_chunk(pending).unwrap().commit_all();
        }

        *running.lock().unwrap() = false;
        // Without the guard an oversized frame count indexes past the pre-sized scratch
        // slice and panics in the producer loop, which join() surfaces here.
        thread_handle.join().unwrap();

        assert_eq!(
            consumer.slots(),
            0,
            "oversized callback should not produce any samples"
        );
    }
}
