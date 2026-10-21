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
use std::time::Duration;

use anyhow::{bail, Context, Result};
use audio_core::{LiveStemRing, LoadedAudio};
use ort::session::builder::GraphOptimizationLevel;
use ort::session::Session;
use ort::value::{Tensor, ValueType};
use tracing::{info, warn};

use crate::live::LiveSeparator;
use crate::resample_pcm::resample_interleaved_stereo;

/// Model sample rate.
pub(crate) const SAMPLE_RATE: u32 = 44_100;
/// One model hop (samples per channel).
const HOP: usize = 128;
/// Four output stems.
const STEMS: usize = 4;
/// Producer may run this far ahead of the deck before sleeping.
const MAX_AHEAD: u64 = 2 * SAMPLE_RATE as u64;
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

/// True when the playhead moved like a seek rather than normal playback.
///
/// `last_playhead` is the last observed value, so a producer that is merely
/// behind does not trigger a re-epoch; only a jump beyond the lookahead does.
fn should_reepoch(playhead: u64, last_playhead: Option<u64>, epoch_start: u64) -> bool {
    playhead < epoch_start
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

fn load_session(weights: &Path) -> Result<(Session, Vec<State>)> {
    let builder = Session::builder()
        .context("StemgenRT ort Session::builder")?
        .with_optimization_level(GraphOptimizationLevel::Level3)
        .map_err(|e| anyhow::anyhow!("StemgenRT ort optimization level: {e}"))?
        .with_intra_threads(1)
        .map_err(|e| anyhow::anyhow!("StemgenRT ort intra threads: {e}"))?
        .with_inter_threads(1)
        .map_err(|e| anyhow::anyhow!("StemgenRT ort inter threads: {e}"))?;
    let session = builder
        .with_execution_providers([ort::ep::CPU::default().build()])
        .map_err(|e| anyhow::anyhow!("StemgenRT ort CPU EP: {e}"))?
        .commit_from_file(weights)
        .with_context(|| format!("load StemgenRT ONNX {}", weights.display()))?;

    let mut states = Vec::new();
    for input in session.inputs() {
        if input.name() == "audio_chunk" {
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
        states = states.len(),
        path = %weights.display(),
        "StemgenRT live: session ready"
    );
    Ok((session, states))
}

/// Run one 128-frame hop. Returns the separated previous hop, `[stem][interleaved]`.
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
        "audio_chunk".into(),
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
        let name = format!("next_{}", state.name);
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

    let (mut session, mut states) = match load_session(weights) {
        Ok(loaded) => loaded,
        Err(err) => {
            warn!(error = %format!("{err:#}"), "StemgenRT live: session failed");
            ring.close();
            return;
        }
    };

    let mut frame = 0u64;
    let mut publish_at = 0u64;
    let mut epoch_start = 0u64;
    let mut first = true;
    let mut flushed = false;
    let mut last_playhead: Option<u64> = None;
    let mut audio_chunk = vec![0.0f32; 2 * HOP];

    loop {
        if stop.load(Ordering::Acquire) || ring.is_closed() {
            break;
        }

        let playhead = ring.playhead().min(total);

        // Distinguish a seek from ordinary producer lag: a forward jump is
        // measured against the last observed playhead, not against the
        // published end. The latter would re-arm immediately after a re-epoch
        // (the warm-up and lookahead are both two seconds) and reset forever.
        let seek = should_reepoch(playhead, last_playhead, epoch_start);
        last_playhead = Some(playhead);
        if seek {
            let start = playhead.saturating_sub(WARMUP);
            for state in states.iter_mut() {
                state.data.fill(0.0);
            }
            ring.reset(start);
            frame = start;
            publish_at = start;
            epoch_start = start;
            first = true;
            flushed = false;
            info!(frame = start, playhead, "StemgenRT live: epoch start");
            continue;
        }

        if publish_at.saturating_sub(playhead) >= MAX_AHEAD {
            thread::sleep(POLL);
            continue;
        }

        if frame >= total {
            if !flushed {
                // One zero hop flushes the final pending hop. Skip it when the
                // stream never produced a pending output.
                if !first {
                    audio_chunk.fill(0.0);
                    match run_hop(&mut session, &mut states, &audio_chunk) {
                        Ok(separated) => {
                            let valid = ((total - publish_at) as usize).min(HOP);
                            if valid > 0 {
                                publish(ring, &separated, publish_at, valid);
                                publish_at += HOP as u64;
                            }
                        }
                        Err(err) => {
                            warn!(error = %format!("{err:#}"), "StemgenRT live: flush failed");
                            ring.close();
                            break;
                        }
                    }
                }
                flushed = true;
            }
            thread::sleep(POLL);
            continue;
        }

        let frames = ((total - frame) as usize).min(HOP);
        audio_chunk.fill(0.0);
        for i in 0..frames {
            let source = (frame as usize + i) * 2;
            audio_chunk[i] = pcm[source];
            audio_chunk[HOP + i] = pcm[source + 1];
        }

        let separated = match run_hop(&mut session, &mut states, &audio_chunk) {
            Ok(separated) => separated,
            Err(err) => {
                warn!(error = %format!("{err:#}"), "StemgenRT live: separation failed");
                ring.close();
                break;
            }
        };

        if !first {
            let valid = ((total - publish_at) as usize).min(HOP);
            if valid > 0 {
                publish(ring, &separated, publish_at, valid);
                publish_at += HOP as u64;
            }
        } else {
            first = false;
        }

        frame = frame.saturating_add(HOP as u64);
    }

    info!("StemgenRT live: producer stopped");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normal_advance_and_producer_lag_do_not_reepoch() {
        assert!(!should_reepoch(0, None, 0));
        assert!(!should_reepoch(44_100, Some(0), 0));
        assert!(!should_reepoch(88_200, Some(44_100), 0));
        assert!(!should_reepoch(2 * MAX_AHEAD, Some(MAX_AHEAD), 0));
    }

    #[test]
    fn forward_and_backward_seeks_reepoch() {
        assert!(should_reepoch(MAX_AHEAD + 1, Some(0), 0));
        assert!(should_reepoch(10, None, 44_100));
    }
}
