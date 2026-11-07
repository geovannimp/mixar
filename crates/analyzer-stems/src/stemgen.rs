//! StemgenRT live separator: causal 128-sample-hop ONNX with 13 states.
//!
//! The model is a streaming filter, not an offline segment separator. It
//! consumes interleaved stereo at 44.1 kHz and writes the preceding 128-frame
//! hop to a [`LiveStemRing`]. The first forward pass is pre-roll and is
//! discarded, matching the upstream `StreamingSeparator` contract.
//!
//! The producer is intentionally independent of the deck's playback code: it
//! reads the already-decoded track, runs ahead of the published playhead, and
//! re-epochs on seeks. The deck keeps its own dry path, so a partially covered
//! ring simply falls back to the main mix.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};
use audio_core::{LiveStemRing, LoadedAudio};
use ort::session::builder::GraphOptimizationLevel;
use ort::session::Session;
use ort::value::{Tensor, ValueType};
use tracing::{info, warn};

use crate::bench::HopBenchmark;
use crate::live::LiveSeparator;
use crate::resample_pcm::resample_interleaved_stereo;

/// Model sample rate.
pub(crate) const SAMPLE_RATE: u32 = 44_100;
/// Name of the model's audio input tensor (the live producer and the
/// benchmark must feed exactly this name).
pub const STEMGEN_AUDIO_INPUT: &str = "audio_chunk";

/// Prefix of the model's recurrent-state outputs (`next_<state>`).
pub const STEMGEN_STATE_OUTPUT_PREFIX: &str = "next_";
/// One model hop (samples per channel).
pub(crate) const HOP: usize = 128;
/// Four output stems.
const STEMS: usize = 4;
/// Producer may run this far ahead of the deck before sleeping.
///
/// The model runs at roughly 0.9–1.0× realtime, so a short lookahead starves
/// within seconds of playback and the deck stays on its dry mix for the rest of
/// the track. Buffering ahead while the deck is idle turns that cliff into a
/// couple of minutes of stems. The ring bounds it: see [`pacing_ahead`].
const MAX_AHEAD: u64 = 12 * SAMPLE_RATE as u64;

/// Frames the producer may stay ahead of the playhead, bounded by the ring.
///
/// `MAX_AHEAD` alone would evict the track start on a smaller ring, so leave a
/// full window behind the playhead: coverage stays usable for rewinds instead of
/// dropping the frames the deck still has to play.
fn pacing_ahead(ring: &LiveStemRing) -> u64 {
    let capacity = ring.capacity_frames() as u64;
    let window = capacity.saturating_sub(2 * HOP as u64) / 2;
    MAX_AHEAD.min(window.max(HOP as u64))
}
/// Analysis context before a seek epoch. Long enough to refill the model's
/// audio histories and most of its recurrent state; exact offline parity is
/// only guaranteed from a track start.
const WARMUP: u64 = 2 * SAMPLE_RATE as u64;
/// Poll interval while pacing or parked at EOF.
const POLL: Duration = Duration::from_millis(5);

struct State {
    name: String,
    shape: Vec<usize>,
    data: Vec<f32>,
}

/// One separation hop: audio in, the previous hop's four stems out.
///
/// The producer loop is generic over this so CI can drive it with a stub
/// instead of the 32 MB StemgenRT artifact.
trait HopRunner {
    fn run(&mut self, audio: &[f32]) -> Result<[Vec<f32>; STEMS]>;

    /// Clear the carried recurrent state (seek / re-epoch).
    fn reset(&mut self);

    /// Execution provider this runner was built with (for throughput logs).
    fn ep_label(&self) -> &'static str {
        "stub"
    }
}

/// True when the playhead moved like a seek rather than normal playback.
///
/// `last_playhead` is the last observed value, so a producer that is merely
/// behind does not trigger a re-epoch; only a jump beyond the lookahead does.
/// A backward seek is detected against the resident coverage start (and the
/// epoch origin), so rewinding out of the published window re-epochs instead of
/// parking the producer behind the deck.
fn should_reepoch(
    playhead: u64,
    last_playhead: Option<u64>,
    epoch_start: u64,
    coverage_start: Option<u64>,
) -> bool {
    playhead < epoch_start
        || coverage_start.is_some_and(|start| playhead < start)
        || last_playhead.is_some_and(|previous| playhead > previous.saturating_add(MAX_AHEAD))
}

/// Start a StemgenRT producer for `audio` into `ring`.
///
/// `weights` must already exist; this function never downloads. The ring rate
/// must be 44.1 kHz (the model rate) and the audio must be stereo.
pub fn start_live_separator(
    weights: PathBuf,
    audio: Arc<LoadedAudio>,
    ring: Arc<LiveStemRing>,
) -> Result<LiveSeparator> {
    if audio.channels != 2 {
        bail!(
            "StemgenRT live separation requires interleaved stereo (got {} channels)",
            audio.channels
        );
    }
    if audio.sample_rate == 0 {
        bail!("StemgenRT live separation requires a non-zero sample rate");
    }
    if ring.rate() != SAMPLE_RATE {
        bail!(
            "StemgenRT live ring rate must be {SAMPLE_RATE} (got {})",
            ring.rate()
        );
    }

    let stop = Arc::new(AtomicBool::new(false));
    let thread_stop = Arc::clone(&stop);
    let thread_ring = Arc::clone(&ring);
    let handle = thread::Builder::new()
        .name("stemgen-live".into())
        .spawn(move || {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                run(&weights, &audio, &thread_ring, &thread_stop);
            }));
            if let Err(payload) = result {
                let message = payload
                    .downcast_ref::<&str>()
                    .map(|s| (*s).to_owned())
                    .or_else(|| payload.downcast_ref::<String>().cloned())
                    .unwrap_or_else(|| "unknown panic".to_owned());
                warn!(error = %message, "StemgenRT live: producer panicked");
                thread_ring.close();
            }
        })
        .map_err(|e| anyhow::anyhow!("spawn StemgenRT live thread: {e}"))?;

    Ok(LiveSeparator::from_parts(stop, ring, handle))
}

fn load_session(weights: &Path) -> Result<(Session, Vec<State>, &'static str)> {
    // `load-dynamic` (Android) resolves the runtime here; the cascade probes it.
    crate::session::environment()?;

    let mut last_err = None;
    for (label, ep) in crate::ep::preferred_live_ep_cascade() {
        match commit_session(weights, label, ep) {
            Ok(loaded) => return Ok(loaded),
            Err(err) => {
                warn!(
                    ep = label,
                    error = %format!("{err:#}"),
                    "StemgenRT live: EP session failed; trying next"
                );
                last_err = Some(err);
            }
        }
    }
    Err(last_err
        .unwrap_or_else(|| anyhow::anyhow!("no ORT execution provider could load StemgenRT")))
}

/// Intra-op thread count for the live CPU session: half the cores, clamped to
/// 1..=4. The graph is mostly sequential, so more threads hurt (on a Galaxy S23
/// eight threads measured 2.5× the per-hop cost of four).
pub fn live_intra_threads() -> usize {
    std::thread::available_parallelism()
        .map(|n| (n.get() / 2).clamp(1, 4))
        .unwrap_or(1)
}

/// Effective intra-op thread count for a session on `label`.
///
/// XNNPACK runs its own intra-op pool, so a second ORT pool only contends and
/// it always gets one thread. Every other EP gets `requested` or the live
/// default ([`live_intra_threads`]: half the cores, capped at four, because the
/// graph is mostly sequential — measured 0.88x one thread, 1.00x four). A
/// requested count the policy would ignore is rejected so a caller cannot
/// report a thread budget it did not measure.
pub fn intra_threads_for(label: &str, requested: Option<usize>) -> Result<usize> {
    if label == crate::XNNPACK_EP_LABEL {
        if let Some(count) = requested {
            if count != 1 {
                bail!("xnnpack runs its own intra-op pool; pass threads=1 (got {count})");
            }
        }
        return Ok(1);
    }
    Ok(requested.unwrap_or_else(live_intra_threads))
}

fn commit_session(
    weights: &Path,
    label: &'static str,
    ep: ort::ep::ExecutionProviderDispatch,
) -> Result<(Session, Vec<State>, &'static str)> {
    let intra_threads = intra_threads_for(label, None)?;
    let builder = Session::builder(crate::session::environment()?)
        .context("StemgenRT ort Session::builder")?
        .with_optimization_level(GraphOptimizationLevel::Level3)
        .map_err(|e| anyhow::anyhow!("StemgenRT ort optimization level: {e}"))?
        .with_intra_threads(intra_threads)
        .map_err(|e| anyhow::anyhow!("StemgenRT ort intra threads: {e}"))?;
    let session = builder
        .with_inter_threads(1)
        .map_err(|e| anyhow::anyhow!("StemgenRT ort inter threads: {e}"))?
        .with_execution_providers([ep])
        .map_err(|e| anyhow::anyhow!("StemgenRT ort register EP '{label}': {e}"))?
        .commit_from_file(weights)
        .with_context(|| format!("load StemgenRT ONNX {}", weights.display()))?;

    let mut states = Vec::new();
    for input in session.inputs() {
        if input.name() == STEMGEN_AUDIO_INPUT {
            continue;
        }
        let ValueType::Tensor { shape, .. } = input.dtype() else {
            bail!("StemgenRT state '{}' is not a tensor", input.name());
        };
        let dims: Vec<usize> = shape
            .iter()
            .map(|d| if *d > 0 { *d as usize } else { 1 })
            .collect();
        let len: usize = dims.iter().product();
        if len == 0 {
            bail!("StemgenRT state '{}' has zero elements", input.name());
        }
        states.push(State {
            name: input.name().to_owned(),
            shape: dims,
            data: vec![0.0; len],
        });
    }
    if states.is_empty() {
        bail!("StemgenRT ONNX has no recurrent states");
    }
    if !session
        .outputs()
        .iter()
        .any(|output| output.name() == "separated_chunk")
    {
        bail!("StemgenRT ONNX is missing 'separated_chunk'");
    }

    info!(
        ep = label,
        states = states.len(),
        path = %weights.display(),
        "StemgenRT live: session ready"
    );
    Ok((session, states, label))
}

/// The real model: one ONNX session plus its carried recurrent states.
struct OnnxRunner {
    session: Session,
    states: Vec<State>,
    ep_label: &'static str,
}

impl HopRunner for OnnxRunner {
    /// Run one 128-frame hop. Returns the separated previous hop, `[stem][interleaved]`.
    fn run(&mut self, audio: &[f32]) -> Result<[Vec<f32>; STEMS]> {
        run_hop(&mut self.session, &mut self.states, audio)
    }

    fn reset(&mut self) {
        for state in self.states.iter_mut() {
            state.data.fill(0.0);
        }
    }

    fn ep_label(&self) -> &'static str {
        self.ep_label
    }
}

/// Run one 128-frame hop against the ONNX graph.
fn run_hop(
    session: &mut Session,
    states: &mut [State],
    audio: &[f32],
) -> Result<[Vec<f32>; STEMS]> {
    if audio.len() != 2 * HOP {
        bail!("StemgenRT hop must be {HOP} stereo frames");
    }

    let mut inputs: Vec<(String, Tensor<f32>)> = Vec::with_capacity(states.len() + 1);
    inputs.push((
        STEMGEN_AUDIO_INPUT.into(),
        Tensor::from_array(([1usize, 2, HOP], audio.to_vec())).context("StemgenRT audio tensor")?,
    ));
    for state in states.iter() {
        inputs.push((
            state.name.clone(),
            Tensor::from_array((state.shape.clone(), state.data.clone()))
                .with_context(|| format!("StemgenRT state tensor '{}'", state.name))?,
        ));
    }

    let outputs = session.run(inputs).context("StemgenRT ort run")?;
    let separated = outputs
        .get("separated_chunk")
        .ok_or_else(|| anyhow::anyhow!("StemgenRT output missing 'separated_chunk'"))?;
    let (shape, data) = separated
        .try_extract_tensor::<f32>()
        .context("extract StemgenRT output")?;
    let dims: Vec<usize> = shape.iter().map(|d| *d as usize).collect();
    if dims != [1, STEMS, 2, HOP] {
        bail!("unexpected StemgenRT output shape {dims:?}");
    }

    let mut out: [Vec<f32>; STEMS] = std::array::from_fn(|_| vec![0.0; HOP * 2]);
    for (stem, stem_out) in out.iter_mut().enumerate() {
        for channel in 0..2 {
            let base = (stem * 2 + channel) * HOP;
            for frame in 0..HOP {
                stem_out[frame * 2 + channel] = data[base + frame];
            }
        }
    }

    for state in states.iter_mut() {
        let name = format!("{}{}", STEMGEN_STATE_OUTPUT_PREFIX, state.name);
        let value = outputs
            .get(&name)
            .ok_or_else(|| anyhow::anyhow!("StemgenRT output missing '{name}'"))?;
        let (_, next) = value
            .try_extract_tensor::<f32>()
            .with_context(|| format!("extract StemgenRT state '{name}'"))?;
        if next.len() != state.data.len() {
            bail!(
                "StemgenRT state '{}' changed size: {} != {}",
                state.name,
                next.len(),
                state.data.len()
            );
        }
        state.data.copy_from_slice(next);
    }

    Ok(out)
}

/// Benchmark a StemgenRT artifact: load a private CPU session, run a warm-up,
/// then time `hops` forward passes.
///
/// This never touches the live ring, the producer, or playback state, so it is
/// safe to run while the engine plays (but never from the audio callback).
pub(crate) fn benchmark(
    weights: &Path,
    hops: u32,
    warmup: u32,
    on_progress: Option<&dyn Fn(u32, u32)>,
) -> Result<HopBenchmark> {
    if hops == 0 {
        bail!("StemgenRT benchmark needs at least one timed hop");
    }
    let (session, states, ep_label) = load_session(weights)?;
    let mut runner = OnnxRunner {
        session,
        states,
        ep_label,
    };
    measure_hops(&mut runner, hops, warmup, on_progress)
}

/// Time `hops` forward passes after `warmup` discarded ones.
///
/// Generic over the hop runner so tests drive it with the same stub as the
/// producer tests instead of the 32 MB artifact.
fn measure_hops(
    runner: &mut impl HopRunner,
    hops: u32,
    warmup: u32,
    on_progress: Option<&dyn Fn(u32, u32)>,
) -> Result<HopBenchmark> {
    let total = hops.saturating_add(warmup);
    // A deterministic ramp keeps every hop's input non-constant, so the graph
    // cannot take a short-circuit and the recurrent states carry real values.
    let mut audio = vec![0.0f32; 2 * HOP];
    for (index, sample) in audio.iter_mut().enumerate() {
        *sample = (index as f32 / (2 * HOP) as f32) * 0.02 - 0.01;
    }

    let mut run_and_report = |completed: u32| -> Result<()> {
        runner.run(&audio)?;
        if let Some(progress) = on_progress {
            progress(completed, total);
        }
        Ok(())
    };

    for done in 0..warmup {
        run_and_report(done + 1)?;
    }

    let start = Instant::now();
    for done in 0..hops {
        run_and_report(warmup + done + 1)?;
    }
    Ok(HopBenchmark::from_elapsed(start.elapsed(), hops))
}

/// Publish one hop's stems and advance the published end.
///
/// Returns the new published end; the caller owns the epoch state.
fn publish_hop(
    ring: &LiveStemRing,
    separated: &[Vec<f32>; STEMS],
    publish_at: &mut u64,
    total: u64,
) {
    let valid = ((total - *publish_at) as usize).min(HOP);
    if valid > 0 {
        publish(ring, separated, *publish_at, valid);
        *publish_at += HOP as u64;
    }
}

fn publish(ring: &LiveStemRing, separated: &[Vec<f32>; STEMS], start: u64, valid: usize) {
    // Model output order is Drums, Bass, Vocals, Other; the ring is
    // Drums, Bass, Other, Vocals (Mixar/NI order).
    let mut stems: [Vec<f32>; STEMS] = std::array::from_fn(|_| vec![0.0; HOP * 2]);
    for (out_stem, ring_stem) in [(0usize, 0usize), (1, 1), (3, 2), (2, 3)] {
        stems[ring_stem][..valid * 2].copy_from_slice(&separated[out_stem][..valid * 2]);
    }
    ring.publish(start, valid, &[&stems[0], &stems[1], &stems[2], &stems[3]]);
}

fn run(weights: &Path, audio: &LoadedAudio, ring: &LiveStemRing, stop: &AtomicBool) {
    let resampled;
    let pcm: &[f32] = if audio.sample_rate == SAMPLE_RATE {
        &audio.samples
    } else {
        match resample_interleaved_stereo(&audio.samples, audio.sample_rate, SAMPLE_RATE) {
            Ok(pcm) => {
                resampled = pcm;
                &resampled
            }
            Err(err) => {
                warn!(error = %err, "StemgenRT live: resample failed");
                ring.close();
                return;
            }
        }
    };

    let total = (pcm.len() / 2) as u64;
    if total == 0 {
        ring.close();
        return;
    }

    let (session, states, ep_label) = match load_session(weights) {
        Ok(loaded) => loaded,
        Err(err) => {
            warn!(error = %format!("{err:#}"), "StemgenRT live: session failed");
            ring.close();
            return;
        }
    };
    let mut runner = OnnxRunner {
        session,
        states,
        ep_label,
    };
    let mut state = ProducerState::new();

    pump(&mut runner, pcm, total, ring, stop, &mut state, u64::MAX);
}

/// Drive the producer loop: separate hops ahead of the playhead and publish
/// them into the ring. Generic over the hop runner so tests can substitute a
/// stub for the ONNX model.
/// Producer epoch state: where the stream is, what is published, and what
/// the last observed playhead was.
#[derive(Default)]
struct ProducerState {
    frame: u64,
    publish_at: u64,
    epoch_start: u64,
    first: bool,
    flushed: bool,
    last_playhead: Option<u64>,
}

impl ProducerState {
    fn new() -> Self {
        Self {
            first: true,
            ..Self::default()
        }
    }
}

/// Rolling model-throughput meter: averaged hop cost and the realtime factor
/// (audio seconds produced per wall second of inference). A factor below 1.0
/// means the producer cannot stay ahead and the deck will fall back to the dry
/// mix; report it every few seconds so device logs answer "is it keeping up?".
struct Throughput {
    run: Duration,
    hops: u64,
    since: Instant,
}

impl Throughput {
    fn new() -> Self {
        Self {
            run: Duration::ZERO,
            hops: 0,
            since: Instant::now(),
        }
    }

    fn record(&mut self, elapsed: Duration) {
        self.run += elapsed;
        self.hops += 1;
    }

    fn report(&mut self, runner: &impl HopRunner) {
        if self.hops == 0 || self.since.elapsed() < Duration::from_secs(5) {
            return;
        }
        let per_hop = self.run.as_secs_f64() / self.hops as f64;
        let budget = HOP as f64 / SAMPLE_RATE as f64;
        let realtime = if per_hop > 0.0 { budget / per_hop } else { 0.0 };
        // One field set for both rows: the warn and info reports stay
        // greppable with the same keys and cannot drift apart.
        macro_rules! report {
            ($level:ident, $message:literal) => {
                tracing::$level!(
                    ep = runner.ep_label(),
                    hops = self.hops,
                    avg_hop_ms = per_hop * 1000.0,
                    realtime,
                    $message
                )
            };
        }
        if realtime < 1.0 {
            report!(warn, "StemgenRT live: producer is slower than realtime");
        } else {
            report!(info, "StemgenRT live: throughput");
        }
        self.run = Duration::ZERO;
        self.hops = 0;
        self.since = Instant::now();
    }
}

/// Run one hop through the throughput meter. Both the steady-state and the
/// flush path go through here so their timing and recording cannot drift.
fn timed_run(
    runner: &mut impl HopRunner,
    throughput: &mut Throughput,
    audio: &[f32],
) -> Result<[Vec<f32>; STEMS]> {
    let started = Instant::now();
    let result = runner.run(audio);
    throughput.record(started.elapsed());
    result
}

fn pump(
    runner: &mut impl HopRunner,
    pcm: &[f32],
    total: u64,
    ring: &LiveStemRing,
    stop: &AtomicBool,
    state: &mut ProducerState,
    max_hops: u64,
) {
    let ProducerState {
        frame,
        publish_at,
        epoch_start,
        first,
        flushed,
        last_playhead,
    } = state;
    let mut hops = 0u64;
    let mut audio_chunk = vec![0.0f32; 2 * HOP];
    let mut throughput = Throughput::new();

    loop {
        if stop.load(Ordering::Acquire) || ring.is_closed() {
            break;
        }
        if hops >= max_hops {
            break;
        }
        hops += 1;

        let playhead = ring.playhead().min(total);

        // Distinguish a seek from ordinary producer lag: a forward jump is
        // measured against the last observed playhead, not against the
        // published end. The latter would re-arm immediately after a re-epoch
        // (the warm-up and lookahead are both two seconds) and reset forever.
        let coverage_start = ring.coverage().map(|(start, _)| start);
        let seek = should_reepoch(playhead, *last_playhead, *epoch_start, coverage_start);
        *last_playhead = Some(playhead);
        if seek {
            let start = playhead.saturating_sub(WARMUP);
            runner.reset();
            ring.reset(start);
            *frame = start;
            *publish_at = start;
            *epoch_start = start;
            *first = true;
            *flushed = false;
            info!(frame = start, playhead, "StemgenRT live: epoch start");
            continue;
        }

        if publish_at.saturating_sub(playhead) >= pacing_ahead(ring) {
            thread::sleep(POLL);
            continue;
        }

        if *frame >= total {
            if !*flushed {
                // One zero hop flushes the final pending hop. Skip it when the
                // stream never produced a pending output.
                if !*first {
                    audio_chunk.fill(0.0);
                    let result = timed_run(runner, &mut throughput, &audio_chunk);
                    match result {
                        Ok(separated) => publish_hop(ring, &separated, publish_at, total),
                        Err(err) => {
                            warn!(error = %format!("{err:#}"), "StemgenRT live: flush failed");
                            ring.close();
                            break;
                        }
                    }
                    throughput.report(runner);
                }
                *flushed = true;
            }
            thread::sleep(POLL);
            continue;
        }

        let frames = ((total - *frame) as usize).min(HOP);
        audio_chunk.fill(0.0);
        for i in 0..frames {
            let source = (*frame as usize + i) * 2;
            audio_chunk[i] = pcm[source];
            audio_chunk[HOP + i] = pcm[source + 1];
        }

        let separated = match timed_run(runner, &mut throughput, &audio_chunk) {
            Ok(separated) => separated,
            Err(err) => {
                warn!(error = %format!("{err:#}"), "StemgenRT live: separation failed");
                ring.close();
                break;
            }
        };
        throughput.report(runner);

        if !*first {
            publish_hop(ring, &separated, publish_at, total);
        } else {
            *first = false;
        }

        *frame = frame.saturating_add(HOP as u64);
    }

    info!("StemgenRT live: producer stopped");
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Stub separator: marks every hop with its call index so the test can
    /// tell which hop landed in the ring.
    struct StubRunner {
        calls: usize,
        resets: usize,
    }

    impl HopRunner for StubRunner {
        fn run(&mut self, _audio: &[f32]) -> Result<[Vec<f32>; STEMS]> {
            self.calls += 1;
            let mut out: [Vec<f32>; STEMS] = std::array::from_fn(|_| vec![0.0; HOP * 2]);
            out[2].fill(self.calls as f32);
            Ok(out)
        }

        fn reset(&mut self) {
            self.resets += 1;
        }
    }

    #[test]
    fn benchmark_times_only_the_timed_hops_and_reports_progress() {
        let mut runner = StubRunner {
            calls: 0,
            resets: 0,
        };
        let progress = std::cell::RefCell::new(Vec::new());
        let benchmark = measure_hops(
            &mut runner,
            5,
            3,
            Some(&|completed, total| {
                progress.borrow_mut().push((completed, total));
            }),
        )
        .expect("stub hops succeed");
        let progress = progress.into_inner();

        // Warm-up is discarded from the measurement but still runs the model.
        assert_eq!(runner.calls, 8);
        assert_eq!(benchmark.hops, 5);
        assert!(
            benchmark.avg_hop_ms >= 0.0,
            "average hop must be non-negative"
        );
        assert!(
            progress.len() == 8 && progress.iter().all(|(_, total)| *total == 8),
            "progress covers warm-up and timed hops: {progress:?}"
        );
        assert_eq!(progress.last(), Some(&(8, 8)));
    }

    #[test]
    fn stub_producer_publishes_coverage_and_resets_on_seek() {
        // Long enough that a playhead past the lookahead is inside the track.
        let frames = 3 * MAX_AHEAD as usize;
        let pcm: Vec<f32> = (0..frames * 2).map(|i| i as f32 * 0.01).collect();
        let ring = LiveStemRing::new(SAMPLE_RATE, 1 << 19);
        ring.set_playhead(0);
        let stop = AtomicBool::new(false);
        let mut runner = StubRunner {
            calls: 0,
            resets: 0,
        };
        let mut state = ProducerState::new();

        pump(
            &mut runner,
            &pcm,
            frames as u64,
            &ring,
            &stop,
            &mut state,
            8,
        );

        // The first hop is pre-roll; the second hop publishes hop 0.
        let (start, end) = ring.coverage().expect("coverage");
        assert_eq!(start, 0);
        assert!(end > 0);
        // The stub marks hop n with n + 1, so published hop 0 carries 2.0.
        assert_eq!(ring.frame(3, 0), (2.0, 2.0));

        // A jump beyond the lookahead is a seek: the producer re-epochs and
        // clears its recurrent state.
        ring.set_playhead(MAX_AHEAD + 1);
        pump(
            &mut runner,
            &pcm,
            frames as u64,
            &ring,
            &stop,
            &mut state,
            4,
        );
        assert!(runner.resets >= 1);
    }

    #[test]
    fn publish_maps_model_order_into_ring_coverage() {
        let ring = LiveStemRing::new(SAMPLE_RATE, 1 << 19);
        let separated: [Vec<f32>; STEMS] = std::array::from_fn(|stem| {
            let value = (stem + 1) as f32;
            vec![value; HOP * 2]
        });
        publish(&ring, &separated, 100, HOP);
        assert_eq!(ring.coverage(), Some((100, 100 + HOP as u64)));
        // Model order drums, bass, vocals, other -> ring drums, bass, other, vocals.
        assert_eq!(ring.frame(0, 100), (1.0, 1.0));
        assert_eq!(ring.frame(1, 100), (2.0, 2.0));
        assert_eq!(ring.frame(2, 100), (4.0, 4.0));
        assert_eq!(ring.frame(3, 100), (3.0, 3.0));
    }

    #[test]
    fn normal_advance_and_producer_lag_do_not_reepoch() {
        assert!(!should_reepoch(0, None, 0, None));
        assert!(!should_reepoch(44_100, Some(0), 0, None));
        assert!(!should_reepoch(88_200, Some(44_100), 0, None));
        assert!(!should_reepoch(2 * MAX_AHEAD, Some(MAX_AHEAD), 0, None));
    }

    #[test]
    fn forward_and_backward_seeks_reepoch() {
        assert!(should_reepoch(MAX_AHEAD + 1, Some(0), 0, None));
        assert!(should_reepoch(10, None, 44_100, None));
        // Rewinding out of the resident coverage must re-epoch, even though the
        // epoch origin is at zero.
        assert!(should_reepoch(10, Some(30_000), 0, Some(20_000)));
        // Staying inside the coverage is normal playback.
        assert!(!should_reepoch(25_000, Some(30_000), 0, Some(20_000)));
    }
}
