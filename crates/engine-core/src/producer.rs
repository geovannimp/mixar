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
}

/// Frames one DSP chunk becomes on a device running at `device_rate`.
///
/// The DSP renders whole chunks at its own rate; a device on another rate consumes a
/// proportional frame count. Rounded to nearest so the per-chunk figure does not drift.
pub(crate) fn resampled_chunk_frames(chunk_frames: u32, dsp_rate: u32, device_rate: u32) -> u32 {
    if dsp_rate == 0 || device_rate == 0 {
        return chunk_frames.max(1);
    }
    let numerator = u64::from(chunk_frames) * u64::from(device_rate) + u64::from(dsp_rate) / 2;
    let frames = numerator / u64::from(dsp_rate);
    frames.max(1) as u32
}

/// Converts one device's stream from the DSP rate to the rate that device opened at.
///
/// The DSP renders in whole chunks at one rate; a device on another rate needs a
/// different frame count per chunk, so the resampler keeps its own history and
/// `pending` holds the input it has not taken yet (the fixed output chunk consumes
/// 511 or 512 input frames per chunk at 48 kHz → 44.1 kHz). Runs on the producer
/// thread, never in the audio callback.
pub(crate) struct DeviceRateConverter {
    resampler: Box<dyn resampler::Resampler>,
    channels: usize,
    /// Interleaved DSP-rate frames the resampler has not consumed yet.
    pending: Vec<Sample>,
    /// One device chunk of output scratch per resampler call.
    out: Vec<Sample>,
    /// Device-rate frames produced but not yet written to the ring.
    ready: Vec<Sample>,
}

impl DeviceRateConverter {
    pub(crate) fn new(
        dsp_rate: u32,
        device_rate: u32,
        channels: u16,
        chunk_frames: u32,
    ) -> Result<Self> {
        let channels = usize::from(channels.max(1));
        // Fixed input: the DSP delivers whole chunks, so the output frame count
        // carries the fractional ratio instead of rounding it away.
        let resampler = resampler::create_device_resampler(
            dsp_rate,
            device_rate,
            channels,
            chunk_frames as usize,
        )?;
        // A variable-output step emits whole internal blocks, so size for the maximum.
        let out_frames = resampler.max_output_frames().max(1);
        Ok(Self {
            resampler,
            channels,
            pending: Vec::with_capacity(channels * (chunk_frames as usize + out_frames)),
            out: vec![0.0; channels * out_frames],
            ready: Vec::with_capacity(channels * out_frames),
        })
    }

    /// Feed one DSP-rate chunk and convert everything the resampler can take now.
    ///
    /// The resampler consumes the fixed input chunk and emits however many frames the
    /// ratio requires (470 or 471 per 512-frame chunk at 48 kHz → 44.1 kHz), so the
    /// stream carries the fractional ratio instead of running ~0.09% slow and
    /// draining the ring until it underruns.
    pub(crate) fn convert(&mut self, chunk: &[Sample]) {
        self.pending.extend_from_slice(chunk);
        loop {
            let need_in = self.resampler.input_frames_next().max(1);
            if self.pending.len() / self.channels < need_in {
                break;
            }
            let (out_samples, consumed_frames) =
                self.resampler
                    .process(&self.pending, &mut self.out, self.channels);
            if consumed_frames > 0 {
                self.pending.drain(..consumed_frames * self.channels);
            }
            if out_samples == 0 {
                break;
            }
            self.ready.extend_from_slice(&self.out[..out_samples]);
        }
    }

    /// Device-rate frames waiting for ring space.
    pub(crate) fn ready_frames(&self) -> usize {
        self.ready.len() / self.channels
    }

    /// Device-rate samples waiting for ring space.
    pub(crate) fn ready(&self) -> &[Sample] {
        &self.ready
    }

    /// Drop the samples the ring accepted.
    pub(crate) fn consume_ready(&mut self, samples: usize) {
        self.ready.drain(..samples.min(self.ready.len()));
    }
}

/// Ring buffer: spec §5.2 — preallocate N × frames_per_buffer (N ≥ 8) to tolerate producer jitter.
///
/// Sized in interleaved samples for `channels`, so multi-channel device plans (e.g. a master +
/// cue bus sharing one 4-channel device) reuse the same helper as a plain stereo bus.
pub(crate) fn create_device_ring_buffer(
    buffer_size: u32,
    channels: u16,
) -> (Producer<Sample>, Consumer<Sample>) {
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

    (producer, consumer)
}

/// Start every opened device stream, then wait for the pacing (first/master) device to run its
/// first callback so startup reports a live device before returning.
pub(crate) fn start_device_streams(
    streams: &mut [Box<dyn AudioStream>],
    dsp_chunk_frames: u32,
    pacing_callback_frames_atomic: Option<Arc<AtomicU32>>,
) -> Result<()> {
    for stream in streams.iter_mut() {
        stream.start()?;
    }

    if let Some(frames_atomic) = pacing_callback_frames_atomic {
        wait_for_callback_frames(frames_atomic, dsp_chunk_frames)?;
    }

    Ok(())
}

/// Wait for the audio device to report its callback frame count.
///
/// The count only has to exist, not to match `dsp_chunk_frames`: the backend decides the callback
/// period (WASAPI shared mode fixes it at the audio engine's ~10 ms, i.e. 480 frames at 48 kHz,
/// which is not even a multiple of the DSP chunk) while the DSP keeps running in whole
/// `dsp_chunk_frames` chunks fed through the ring buffer. Rejecting a differing count would make
/// those devices unopenable.
pub(crate) fn wait_for_callback_frames(
    frames_atomic: Arc<AtomicU32>,
    dsp_chunk_frames: u32,
) -> Result<()> {
    const TIMEOUT: Duration = Duration::from_secs(2);
    let deadline = Instant::now() + TIMEOUT;

    while Instant::now() < deadline {
        let frames = frames_atomic.load(Ordering::Relaxed);
        if frames > 0 {
            if frames == dsp_chunk_frames {
                tracing::info!("Device callback size verified: {} frames", frames);
            } else {
                tracing::info!(
                    "Device callback period is {} frames, DSP chunk stays {} frames (ring buffer decouples them)",
                    frames,
                    dsp_chunk_frames
                );
            }
            return Ok(());
        }
        thread::sleep(Duration::from_millis(1));
    }

    Err(anyhow::anyhow!(
        "Timed out waiting for audio device callback (DSP chunk {} frames)",
        dsp_chunk_frames
    ))
}

/// Run one DSP chunk. On error, log it and zero every output bus so the
/// caller's map/push path emits silence instead of the previous chunk.
fn process_chunk_or_silence(
    dsp: &mut DspEngine,
    frames: u32,
    output_buses: &mut HashMap<BusId, Vec<Sample>>,
) {
    if let Err(e) = dsp.process(frames, output_buses) {
        tracing::error!("DSP processing error: {}", e);
        for bus_buf in output_buses.values_mut() {
            bus_buf.fill(0.0);
        }
    }
}

/// A device whose callback counter has not advanced for this long is treated as unplugged or
/// stalled. Long enough to ride out callback jitter under load; short enough that a dead device
/// does not hold back the others.
const DEVICE_STALL_TIMEOUT: Duration = Duration::from_millis(500);

/// Liveness of one device, derived from its callback counter. Kept pure so the transition is
/// unit-testable.
#[derive(Debug)]
struct DeviceLiveness {
    last_count: u64,
    last_advance: Instant,
}

impl DeviceLiveness {
    fn new(started: Instant) -> Self {
        Self {
            last_count: 0,
            last_advance: started,
        }
    }

    /// Observe the device's callback count at `now`; returns `true` while the device is live.
    ///
    /// The stall clock is seeded at construction, so every device starts live and the producer
    /// can fill rings during stream startup. A device whose callback never fires goes dead after
    /// the timeout and stops holding back the others.
    ///
    /// `now` MUST be one shared timestamp sampled once per producer iteration and passed for
    /// every device: per-device or stale timestamps would compare against each device's own
    /// clock and break the stall deadline uniformly across devices.
    fn observe(&mut self, count: u64, now: Instant) -> bool {
        if count != self.last_count {
            self.last_count = count;
            self.last_advance = now;
        }
        // `checked_duration_since` guards against a backward clock step (suspend/resume, NTP
        // adjustment): a backwards `now` reads as "no time elapsed" rather than panicking.
        now.checked_duration_since(self.last_advance)
            .unwrap_or(Duration::ZERO)
            < DEVICE_STALL_TIMEOUT
    }
}

/// Producer thread loop (spec §5.1: writes decoded/resampled audio into per-device ring buffers).
///
/// The DSP always runs in whole `dsp_chunk_frames` chunks (a multiple of `dasp_graph`'s buffer
/// length). Devices are free to call back in any frame count: the backend's period is not the DSP
/// chunk, and the ring buffer between them absorbs the difference — a device that consumes 480
/// frames per callback simply reads 480 frames out of rings the producer fills 512 frames at a
/// time. Pacing is therefore the pacing ring's fill level, not the device's callback count:
/// production stops once that ring holds two chunks, which ties production to consumption without
/// needing the two sizes to agree.
///
/// Index 0 is the preferred pacing device (`resolve_device_stream_plans` sorts a master-hosting
/// plan first), so it is the "master clock" whenever a master bus is configured. If it stalls or is
/// unplugged, pacing falls over to the next live device. Dead devices are excluded from the room
/// gate and from pushes, so only their own output stops; the remaining devices keep playing.
#[allow(clippy::too_many_arguments)]
pub(crate) fn producer_thread_loop(
    dsp_engine: Arc<Mutex<DspEngine>>,
    mut device_producers: Vec<(DeviceStreamPlan, Producer<Sample>)>,
    mut converters: Vec<Option<DeviceRateConverter>>,
    running: Arc<Mutex<bool>>,
    sample_rate: u32,
    dsp_chunk_frames: usize,
    callback_frames: Vec<Option<Arc<AtomicU32>>>,
    callback_counts: Vec<Arc<AtomicU64>>,
    transport_events: Arc<Mutex<Vec<TransportEvent>>>,
) {
    tracing::info!(
        "Producer thread started ({} device stream(s), dsp_chunk_frames={}, sample_rate={})",
        device_producers.len(),
        dsp_chunk_frames,
        sample_rate
    );

    // Every scratch/output buffer is pre-sized to the DSP chunk so the steady-state loop never
    // allocates. The chunk does not depend on any device, so it cannot be exceeded at runtime.
    let chunk_frames = dsp_chunk_frames;

    let mut output_buses = HashMap::new();
    output_buses.insert(BusId::new("master"), vec![0.0; chunk_frames * 2]);
    if device_producers
        .iter()
        .any(|(plan, _)| plan.routes.iter().any(|r| r.bus_id.as_str() == "cue"))
    {
        output_buses.insert(BusId::new("cue"), vec![0.0; chunk_frames * 2]);
    }

    let mut device_scratch: Vec<Vec<Sample>> = device_producers
        .iter()
        .map(|(plan, _)| vec![0.0; chunk_frames * plan.channels as usize])
        .collect();

    // Frames each device takes per DSP chunk: equal to the chunk on the DSP clock,
    // proportional to the device rate otherwise (the converter produces that many).
    let device_chunk_frames: Vec<usize> = device_producers
        .iter()
        .map(|(plan, _)| {
            let rate = plan.sample_rate.unwrap_or(sample_rate);
            resampled_chunk_frames(chunk_frames as u32, sample_rate, rate) as usize
        })
        .collect();
    assert_eq!(
        device_chunk_frames.len(),
        device_producers.len(),
        "device_chunk_frames must pair 1:1 with device_producers"
    );
    assert_eq!(
        converters.len(),
        device_producers.len(),
        "converters must pair 1:1 with device_producers"
    );

    let mut pacing_index: Option<usize> = None;
    // Ring capacity is fixed at construction (rtrb rings are not resized), so snapshot it per
    // device up front instead of re-reading it from the pacing producer on every failover.
    let ring_capacities: Vec<usize> = device_producers
        .iter()
        .map(|(_, producer)| producer.buffer().capacity())
        .collect();
    let mut liveness: Vec<DeviceLiveness> = device_producers
        .iter()
        .map(|_| DeviceLiveness::new(Instant::now()))
        .collect();
    let mut live: Vec<bool> = Vec::with_capacity(device_producers.len());

    if device_producers.is_empty() {
        tracing::info!("No device streams; producer thread exiting");
        return;
    }

    // The producer writes each chunk to every device ring and reads `live`/`callback_counts`
    // positionally, so a length mismatch is a caller programming error that would silently
    // drop a device in release builds. These are one-time checks before the loop.
    assert_eq!(
        liveness.len(),
        callback_counts.len(),
        "callback_counts must pair 1:1 with device_producers"
    );
    assert_eq!(
        liveness.len(),
        callback_frames.len(),
        "callback_frames must pair 1:1 with device_producers"
    );
    assert_eq!(
        liveness.len(),
        device_producers.len(),
        "liveness must pair 1:1 with device_producers"
    );

    while *running.lock().expect("running flag mutex poisoned") {
        let buffer_duration = Duration::from_secs_f64(chunk_frames as f64 / sample_rate as f64);

        let now = Instant::now();
        live.clear();
        for (state, count) in liveness.iter_mut().zip(&callback_counts) {
            live.push(state.observe(count.load(Ordering::Relaxed), now));
        }

        // No device is producing callbacks. Back off well past the stall timeout so a dead
        // output device does not spin the producer thread, in short slices that keep shutdown
        // latency bounded by checking `running` between sleeps.
        let Some(pacing_i) = live.iter().position(|&l| l) else {
            let slice = DEVICE_STALL_TIMEOUT / 16;
            for _ in 0..4 {
                if !*running.lock().expect("running flag mutex poisoned") {
                    break;
                }
                thread::sleep(slice);
            }
            continue;
        };

        if pacing_index != Some(pacing_i) {
            // Failover (or recovery back to a lower-index device). Production is bounded by the
            // new pacing ring's fill level, so nothing else has to be re-based here.
            match pacing_index {
                Some(prev_i) => {
                    tracing::info!("Pacing changed: device {} -> device {}", prev_i, pacing_i)
                }
                None => tracing::info!("Pacing started on device {}", pacing_i),
            }
            pacing_index = Some(pacing_i);
        }

        let (pacing_plan, pacing_producer) = &device_producers[pacing_i];
        let pacing_chunk_frames = device_chunk_frames[pacing_i];
        let samples_per_chunk = pacing_chunk_frames * pacing_plan.channels as usize;
        // The ring must hold at least one device callback on top of one DSP chunk: a device whose
        // period exceeds the configured buffer size (WASAPI at small configured sizes) would
        // otherwise underrun on every callback. Two chunks stays the floor at equal sizes.
        let device_frames = callback_frames
            .get(pacing_i)
            .and_then(|frames| frames.as_ref())
            .map(|frames| frames.load(Ordering::Relaxed) as usize)
            .unwrap_or(0);
        let target_frames = (pacing_chunk_frames * 2).max(device_frames + pacing_chunk_frames);
        let pacing_slots = pacing_producer.slots();
        let filled = ring_capacities[pacing_i].saturating_sub(pacing_slots);
        let target_fill = target_frames * pacing_plan.channels as usize;
        if filled >= target_fill || pacing_slots < samples_per_chunk {
            thread::sleep(buffer_duration / 8);
            continue;
        }

        // Every live device must have room for a full chunk (plus anything a
        // converted stream has queued) before DSP runs, so no ring is ever committed
        // a partial chunk. Dead devices are excluded and skipped during the push.
        let all_live_have_room = device_producers.iter().zip(&live).enumerate().all(
            |(i, ((plan, producer), &is_live))| {
                !is_live
                    || producer.slots()
                        >= (device_chunk_frames[i]
                            + converters[i]
                                .as_ref()
                                .map_or(0, DeviceRateConverter::ready_frames))
                            * plan.channels as usize
            },
        );
        if !all_live_have_room {
            thread::sleep(buffer_duration / 8);
            continue;
        }

        {
            let mut dsp = dsp_engine.lock().expect("DSP engine mutex poisoned");
            process_chunk_or_silence(&mut dsp, chunk_frames as u32, &mut output_buses);
            let deck_events = dsp.drain_transport_events();
            if !deck_events.is_empty() {
                let mut queue = transport_events
                    .lock()
                    .expect("transport event queue mutex poisoned");
                queue.extend(
                    deck_events
                        .into_iter()
                        .map(|(deck_id, event)| TransportEvent::from_deck(deck_id, event)),
                );
            }
        }

        // `zip(&live)` keeps this loop panic-free even if `live` were ever shorter than
        // `device_producers` (the assertions above catch that in debug builds).
        for (i, ((plan, producer), &is_live)) in device_producers.iter_mut().zip(&live).enumerate()
        {
            if !is_live {
                continue;
            }
            let channels = plan.channels as usize;
            let needed = chunk_frames * channels;
            map_buses_to_device_buffer(
                chunk_frames,
                channels,
                &plan.routes,
                &output_buses,
                &mut device_scratch[i][..needed],
            );

            // Devices on the DSP clock go straight to the ring; a device opened at
            // another rate is converted first (the converter keeps its own history and
            // queues output the ring could not take yet).
            match converters[i].as_mut() {
                Some(converter) => {
                    converter.convert(&device_scratch[i][..needed]);
                    let pushed = push_ring_samples(producer, converter.ready());
                    converter.consume_ready(pushed);
                }
                None => {
                    push_ring_samples(producer, &device_scratch[i][..needed]);
                }
            }
        }
    }

    tracing::info!("Producer thread stopped");
}

/// Bulk-write interleaved scratch into the ring. Returns how many samples the ring
/// accepted: fewer than `samples.len()` when it is nearly full, so a converted
/// stream can keep the rest queued instead of dropping it.
fn push_ring_samples(producer: &mut Producer<Sample>, samples: &[Sample]) -> usize {
    if samples.is_empty() {
        return 0;
    }

    let n = producer.slots().min(samples.len());
    if n == 0 {
        return 0;
    }
    let Ok(mut chunk) = producer.write_chunk(n) else {
        return 0;
    };
    let (first, second) = chunk.as_mut_slices();
    let first_take = first.len().min(n);
    let second_take = n - first_take;
    first[..first_take].copy_from_slice(&samples[..first_take]);
    second[..second_take].copy_from_slice(&samples[first_take..n]);
    chunk.commit_all();
    n
}

#[cfg(test)]
mod tests {
    use super::*;
    // Arc, Mutex, thread, Duration and the atomics all resolve via `use super::*`.

    #[test]
    fn push_ring_samples_writes_full_chunk() {
        let (mut producer, mut consumer) = RingBuffer::new(16);
        let samples = [0.1, 0.2, 0.3, 0.4];

        assert_eq!(push_ring_samples(&mut producer, &samples), 4);
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
        assert_eq!(push_ring_samples(&mut producer, &[1.0, 2.0, 3.0]), 3);

        let samples = [4.0, 5.0, 6.0];
        // Only one slot was free: the rest stays with the caller.
        assert_eq!(push_ring_samples(&mut producer, &samples), 1);
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
    fn push_ring_samples_returns_zero_when_ring_full() {
        let (mut producer, _consumer) = RingBuffer::new(2);
        assert_eq!(push_ring_samples(&mut producer, &[1.0, 2.0]), 2);
        assert_eq!(push_ring_samples(&mut producer, &[3.0, 4.0]), 0);
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
            preferred: DeviceId::new("pacing"),
            channels: CHANNELS,
            sample_rate: Some(48_000),
            low_latency: false,
            routes: vec![BusRoute {
                bus_id: BusId::new("master"),
                left: 0,
                right: Some(1),
            }],
        };
        let nonpacing_plan = DeviceStreamPlan {
            device: DeviceId::new("nonpacing"),
            preferred: DeviceId::new("nonpacing"),
            channels: CHANNELS,
            sample_rate: Some(48_000),
            low_latency: false,
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
        assert_eq!(
            push_ring_samples(&mut nonpacing_prod, &vec![0.0; prefill]),
            prefill
        );

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
                vec![None, None],
                running_thread,
                48_000,
                BUFFER_FRAMES as usize,
                vec![None, None],
                vec![
                    Arc::new(AtomicU64::new(200)),
                    Arc::new(AtomicU64::new(1000)),
                ],
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

        *running.lock().expect("running flag mutex poisoned") = false;
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
    fn wait_for_callback_frames_accepts_a_device_period_other_than_the_chunk() {
        // WASAPI shared mode reports 480 frames per callback against a 512-frame DSP chunk;
        // startup must not treat that as a misconfiguration (it used to fail the whole start).
        let frames = Arc::new(AtomicU32::new(480));
        wait_for_callback_frames(frames, 512).unwrap();
    }

    #[test]
    fn producer_paces_ring_to_a_device_period_smaller_than_the_chunk() {
        use crate::routing::BusRoute;
        use audio_core::{BusId, DeviceId};
        use engine_dsp::SamplerStripRoute;

        // WASAPI shared mode at 48 kHz: 480 frames per callback, 512-frame DSP chunk.
        const DEVICE_FRAMES: usize = 480;
        const CHUNK_FRAMES: usize = 512;
        const CHANNELS: u16 = 2;

        let plan = DeviceStreamPlan {
            device: DeviceId::new("wasapi-shared"),
            preferred: DeviceId::new("wasapi-shared"),
            channels: CHANNELS,
            sample_rate: Some(48_000),
            low_latency: false,
            routes: vec![BusRoute {
                bus_id: BusId::new("master"),
                left: 0,
                right: Some(1),
            }],
        };

        let (producer, mut consumer) = create_device_ring_buffer(CHUNK_FRAMES as u32, CHANNELS);
        // Start empty so every byte in the ring is produced by the loop under test.
        let prefill = consumer.slots();
        consumer.read_chunk(prefill).unwrap().commit_all();

        let dsp_engine = Arc::new(Mutex::new(DspEngine::new(
            48_000,
            CHUNK_FRAMES as u32,
            1,
            "medium",
            SamplerStripRoute::BeforeStrip,
        )));
        let running = Arc::new(Mutex::new(true));
        let callback_count = Arc::new(AtomicU64::new(0));

        let running_clone = Arc::clone(&running);
        let callback_count_clone = Arc::clone(&callback_count);
        let handle = thread::spawn(move || {
            producer_thread_loop(
                dsp_engine,
                vec![(plan, producer)],
                vec![None],
                running_clone,
                48_000,
                CHUNK_FRAMES,
                vec![Some(Arc::new(AtomicU32::new(DEVICE_FRAMES as u32)))],
                vec![callback_count_clone],
                Arc::new(Mutex::new(Vec::new())),
            );
        });

        // Drive device callbacks that consume less than one DSP chunk each and check the ring
        // never falls behind consumption: production has to follow the ring drain, not the
        // callback count, or a 480-frame device would accumulate the whole ring.
        let want = DEVICE_FRAMES * CHANNELS as usize;
        let mut consumed = 0usize;
        let mut max_slots = 0usize;
        for _ in 0..40 {
            callback_count.fetch_add(1, Ordering::Relaxed);
            let deadline = Instant::now() + Duration::from_millis(100);
            while consumer.slots() < want && Instant::now() < deadline {
                thread::sleep(Duration::from_millis(1));
            }
            let take = consumer.slots().min(want);
            if take > 0 {
                consumer.read_chunk(take).unwrap().commit_all();
                consumed += take;
            }
            max_slots = max_slots.max(consumer.slots());
        }

        *running.lock().expect("running flag mutex poisoned") = false;
        handle.join().unwrap();

        assert!(consumed > 0, "callbacks never received audio");
        let bound = 3 * CHUNK_FRAMES * CHANNELS as usize;
        assert!(
            max_slots <= bound,
            "ring filled to {max_slots} samples while consuming {consumed}; production must stay \
             within ~2 DSP chunks of the device's consumption (bound {bound})"
        );
        assert!(
            consumer.slots() <= bound,
            "ring left {} samples queued, expected at most {bound}",
            consumer.slots()
        );
    }

    #[test]
    fn producer_keeps_a_full_device_period_queued_when_it_exceeds_the_chunk() {
        use crate::routing::BusRoute;
        use audio_core::{BusId, DeviceId};
        use engine_dsp::SamplerStripRoute;

        // A 64-frame configured buffer on WASAPI: the device still calls back every 480 frames, so
        // a two-chunk fill target (128 frames) would underrun on every single callback.
        const DEVICE_FRAMES: usize = 480;
        const CHUNK_FRAMES: usize = 64;
        const CHANNELS: u16 = 2;
        const SAMPLES_PER_CALLBACK: usize = DEVICE_FRAMES * CHANNELS as usize;

        let plan = DeviceStreamPlan {
            device: DeviceId::new("wasapi-small-buffer"),
            preferred: DeviceId::new("wasapi-small-buffer"),
            channels: CHANNELS,
            sample_rate: Some(48_000),
            low_latency: false,
            routes: vec![BusRoute {
                bus_id: BusId::new("master"),
                left: 0,
                right: Some(1),
            }],
        };

        let (producer, mut consumer) = create_device_ring_buffer(CHUNK_FRAMES as u32, CHANNELS);
        // Drain the startup prefill so the fill target alone decides what stays queued.
        let prefill = consumer.slots();
        consumer.read_chunk(prefill).unwrap().commit_all();

        let dsp_engine = Arc::new(Mutex::new(DspEngine::new(
            48_000,
            CHUNK_FRAMES as u32,
            1,
            "medium",
            SamplerStripRoute::BeforeStrip,
        )));
        let running = Arc::new(Mutex::new(true));
        let running_clone = Arc::clone(&running);
        let handle = thread::spawn(move || {
            producer_thread_loop(
                dsp_engine,
                vec![(plan, producer)],
                vec![None],
                running_clone,
                48_000,
                CHUNK_FRAMES,
                vec![Some(Arc::new(AtomicU32::new(DEVICE_FRAMES as u32)))],
                vec![Arc::new(AtomicU64::new(1))],
                Arc::new(Mutex::new(Vec::new())),
            );
        });

        let deadline = Instant::now() + Duration::from_millis(300);
        while consumer.slots() < SAMPLES_PER_CALLBACK && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(1));
        }
        *running.lock().expect("running flag mutex poisoned") = false;
        handle.join().unwrap();

        assert!(
            consumer.slots() >= SAMPLES_PER_CALLBACK,
            "ring holds {} samples but one device callback needs {}; the fill target must cover \
             the device period",
            consumer.slots(),
            SAMPLES_PER_CALLBACK
        );
    }

    #[test]
    fn process_error_zeros_output_buses() {
        use engine_dsp::{DspEngine, SamplerStripRoute};
        use std::collections::HashMap;

        let mut dsp = DspEngine::new(48_000, 64, 2, "medium", SamplerStripRoute::BeforeStrip);
        let mut output_buses = HashMap::new();
        output_buses.insert(BusId::new("master"), vec![0.5_f32; 64 * 2]);

        // 65 frames is not a multiple of dasp_graph::Buffer::LEN (64),
        // so mixer processing returns an error.
        process_chunk_or_silence(&mut dsp, 65, &mut output_buses);

        for (bus_id, buf) in &output_buses {
            assert!(
                buf.iter().all(|&s| s == 0.0),
                "bus {} must be silent after process error",
                bus_id.as_str()
            );
        }
    }

    #[test]
    fn device_stays_live_while_callbacks_advance() {
        let t0 = Instant::now();
        let mut state = DeviceLiveness::new(t0);
        assert!(state.observe(1, t0));
        assert!(state.observe(2, t0 + Duration::from_millis(100)));
        assert!(state.observe(3, t0 + DEVICE_STALL_TIMEOUT));
        assert!(state.observe(4, t0 + DEVICE_STALL_TIMEOUT * 3));
    }

    #[test]
    fn device_live_before_first_callback_then_dead_without_one() {
        let t0 = Instant::now();
        let mut state = DeviceLiveness::new(t0);
        // Slow startup: the clock is seeded at construction, so the device stays live
        // during the startup wait...
        assert!(state.observe(0, t0 + Duration::from_millis(100)));
        // ...and its first callback, even a late one, keeps it live.
        assert!(state.observe(1, t0 + DEVICE_STALL_TIMEOUT));
        // A device whose callback never fires goes dead after the timeout and stops holding
        // the others back.
        let mut never = DeviceLiveness::new(t0);
        assert!(never.observe(0, t0 + Duration::from_millis(100)));
        assert!(!never.observe(0, t0 + DEVICE_STALL_TIMEOUT));
    }

    #[test]
    fn device_goes_dead_after_stall_timeout_and_revives_on_advance() {
        let t0 = Instant::now();
        let mut state = DeviceLiveness::new(t0);
        assert!(state.observe(5, t0));
        assert!(!state.observe(5, t0 + DEVICE_STALL_TIMEOUT));
        assert!(!state.observe(5, t0 + DEVICE_STALL_TIMEOUT * 2));
        assert!(state.observe(6, t0 + DEVICE_STALL_TIMEOUT * 2));
    }

    #[test]
    fn device_that_stops_after_first_advance_goes_dead() {
        let t0 = Instant::now();
        let mut state = DeviceLiveness::new(t0);
        // First advance starts the stall clock...
        assert!(state.observe(7, t0 + Duration::from_millis(100)));
        // ...and a device that then stops advancing goes dead after the timeout.
        assert!(!state.observe(7, t0 + Duration::from_millis(100) + DEVICE_STALL_TIMEOUT));
    }

    #[test]
    fn chunk_frames_scale_with_the_device_rate() {
        // Same clock: unchanged.
        assert_eq!(resampled_chunk_frames(512, 48_000, 48_000), 512);
        // 512 frames @ 48 kHz is 10.67 ms -> 470 frames @ 44.1 kHz.
        assert_eq!(resampled_chunk_frames(512, 48_000, 44_100), 470);
        assert_eq!(resampled_chunk_frames(512, 48_000, 96_000), 1_024);
        assert_eq!(resampled_chunk_frames(512, 48_000, 88_200), 941);
        assert_eq!(resampled_chunk_frames(64, 48_000, 44_100), 59);
        // A degenerate rate never yields a zero-frame chunk.
        assert_eq!(resampled_chunk_frames(512, 0, 44_100), 512);
        assert_eq!(resampled_chunk_frames(512, 48_000, 0), 512);
    }

    #[test]
    fn converter_tracks_the_ratio_and_keeps_the_level() {
        const DSP_RATE: u32 = 48_000;
        const DEVICE_RATE: u32 = 44_100;
        const CHANNELS: u16 = 2;
        const CHUNK: u32 = 512;
        // ~3.5 min of audio: the old fixed-output behaviour drifted ~950 frames here,
        // while the buffered tail of a correct stream stays under one block.
        const CHUNKS: usize = 20_000;

        let mut converter =
            DeviceRateConverter::new(DSP_RATE, DEVICE_RATE, CHANNELS, CHUNK).expect("converter");
        // DC input: a correct resampler returns DC, so the level is easy to check.
        let chunk = vec![0.5f32; CHUNK as usize * CHANNELS as usize];

        let mut produced_frames = 0usize;
        let mut max_backlog_frames = 0usize;
        let mut last_mean = 0.0f32;
        for index in 0..CHUNKS {
            converter.convert(&chunk);
            let ready_len = converter.ready().len();
            assert_eq!(
                ready_len % CHANNELS as usize,
                0,
                "output must be whole interleaved frames"
            );
            produced_frames += ready_len / CHANNELS as usize;
            if index + 1 == CHUNKS {
                last_mean = converter.ready().iter().sum::<f32>() / ready_len as f32;
            }
            max_backlog_frames = max_backlog_frames
                .max(converter.ready_frames() + converter.pending.len() / CHANNELS as usize);
            converter.consume_ready(ready_len);
        }

        // 512 frames @ 48 kHz is 10.667 ms, i.e. exactly 470.4 frames @ 44.1 kHz.
        // The fixed output chunk is 470, so the extra call must land every so often;
        // missing it runs the stream ~0.09% slow and drains the ring within minutes.
        let expected = (CHUNKS as f64 * f64::from(CHUNK) * f64::from(DEVICE_RATE)
            / f64::from(DSP_RATE))
        .round() as usize;
        // What the resampler has buffered internally at the end never exceeds one
        // maximum block, and nothing errors out: the count tracks the exact ratio.
        let drift = produced_frames as i64 - expected as i64;
        assert!(
            drift.abs() <= 400,
            "{produced_frames} frames produced over {CHUNKS} chunks, expected {expected}"
        );
        // The backlog stays near one chunk: conversion keeps pace with the DSP.
        let chunk_frames = CHUNK as usize;
        assert!(
            max_backlog_frames <= 3 * chunk_frames,
            "backlog reached {max_backlog_frames} frames"
        );

        assert!(
            (last_mean - 0.5).abs() < 5e-3,
            "steady-state level drifted to {last_mean}"
        );
    }

    #[test]
    fn converted_device_is_fed_at_its_own_rate() {
        use crate::routing::BusRoute;
        use audio_core::DeviceId;
        use engine_dsp::SamplerStripRoute;

        const CHUNK_FRAMES: u32 = 64; // dasp_graph::Buffer::LEN
        const CHANNELS: u16 = 2;
        const DSP_RATE: u32 = 48_000;
        const CUE_RATE: u32 = 44_100;
        const TARGET_MASTER_FRAMES: usize = 200 * CHUNK_FRAMES as usize;

        let master_plan = DeviceStreamPlan {
            device: DeviceId::new("master-device"),
            preferred: DeviceId::new("master-device"),
            channels: CHANNELS,
            sample_rate: Some(DSP_RATE),
            low_latency: false,
            routes: vec![BusRoute {
                bus_id: BusId::new("master"),
                left: 0,
                right: Some(1),
            }],
        };
        let cue_plan = DeviceStreamPlan {
            device: DeviceId::new("cue-device"),
            preferred: DeviceId::new("cue-device"),
            channels: CHANNELS,
            sample_rate: Some(CUE_RATE),
            low_latency: false,
            routes: vec![BusRoute {
                bus_id: BusId::new("cue"),
                left: 0,
                right: Some(1),
            }],
        };

        let cue_chunk = resampled_chunk_frames(CHUNK_FRAMES, DSP_RATE, CUE_RATE);
        let (master_producer, mut master_consumer) =
            create_device_ring_buffer(CHUNK_FRAMES, CHANNELS);
        let (cue_producer, mut cue_consumer) = create_device_ring_buffer(cue_chunk, CHANNELS);
        // Start empty so every sample counted came from the loop under test.
        for consumer in [&mut master_consumer, &mut cue_consumer] {
            let prefill = consumer.slots();
            consumer.read_chunk(prefill).unwrap().commit_all();
        }

        let dsp_engine = Arc::new(Mutex::new(DspEngine::new(
            DSP_RATE,
            CHUNK_FRAMES,
            1,
            "medium",
            SamplerStripRoute::BeforeStrip,
        )));
        let converter = DeviceRateConverter::new(DSP_RATE, CUE_RATE, CHANNELS, CHUNK_FRAMES)
            .expect("converter");
        let running = Arc::new(Mutex::new(true));
        let running_thread = Arc::clone(&running);
        let handle = thread::spawn(move || {
            producer_thread_loop(
                dsp_engine,
                vec![(master_plan, master_producer), (cue_plan, cue_producer)],
                vec![None, Some(converter)],
                running_thread,
                DSP_RATE,
                CHUNK_FRAMES as usize,
                vec![
                    Some(Arc::new(AtomicU32::new(CHUNK_FRAMES))),
                    Some(Arc::new(AtomicU32::new(cue_chunk))),
                ],
                vec![Arc::new(AtomicU64::new(0)), Arc::new(AtomicU64::new(0))],
                Arc::new(Mutex::new(Vec::new())),
            );
        });

        // Drain both rings so pacing keeps producing; the totals then show the
        // cue stream running at its own rate instead of the DSP clock.
        let mut master_frames = 0usize;
        let mut cue_frames = 0usize;
        let deadline = Instant::now() + Duration::from_secs(10);
        while master_frames < TARGET_MASTER_FRAMES && Instant::now() < deadline {
            let n = master_consumer.slots();
            if n > 0 {
                master_consumer.read_chunk(n).unwrap().commit_all();
                master_frames += n / CHANNELS as usize;
            }
            let m = cue_consumer.slots();
            if m > 0 {
                cue_consumer.read_chunk(m).unwrap().commit_all();
                cue_frames += m / CHANNELS as usize;
            }
            thread::sleep(Duration::from_millis(1));
        }
        *running.lock().unwrap() = false;
        handle.join().expect("producer thread");

        assert!(
            master_frames >= TARGET_MASTER_FRAMES,
            "producer stalled after {master_frames} master frames"
        );
        let ratio = cue_frames as f64 / master_frames as f64;
        let expected = f64::from(CUE_RATE) / f64::from(DSP_RATE);
        assert!(
            (ratio - expected).abs() < 0.03,
            "cue ran at {ratio:.3}x the DSP rate, expected {expected:.3}x"
        );
    }
}
