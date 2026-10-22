//! Live (on-the-fly) stem separation into a bounded ring.
//!
//! The producer mirrors the offline chunk/window math (see
//! [`infer::finalize_window`]), so a live track's stems are bit-identical to a
//! cached separation of the same audio. It runs ahead of the deck's published
//! playhead and never writes files.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use anyhow::{bail, Result};
use audio_core::{LiveStemRing, LoadedAudio};
use tracing::{info, warn};

use crate::infer::{self, CHUNK_STRIDE, SAMPLE_RATE, SOURCES};
use crate::resample_pcm::resample_interleaved_stereo;
use crate::session::{self, SEGMENT_SAMPLES};

/// How far the producer may run ahead of the deck before sleeping.
///
/// Must stay below the ring capacity minus one stride so a wrapped window never
/// overwrites frames at or ahead of the reader.
const MAX_AHEAD_FRAMES: u64 = (CHUNK_STRIDE * 2) as u64;

/// Producer poll interval while pacing or waiting for a seek.
const POLL: Duration = Duration::from_millis(10);

/// Largest safe producer lead for a ring of `capacity` frames.
///
/// A window published after the pace check must still end strictly before
/// `playhead + capacity`; otherwise it would wrap onto frames the reader can
/// still hold. The 4096-frame margin also keeps the interpolation pair safe.
fn max_ahead_frames(capacity: usize) -> u64 {
    MAX_AHEAD_FRAMES.min((capacity as u64).saturating_sub(CHUNK_STRIDE as u64 + 4096))
}

/// Handle to a running live separator.
///
/// Dropping it stops the producer (the deck keeps whatever was already
/// published until it replaces the stems or unloads).
pub struct LiveSeparator {
    stop: Arc<AtomicBool>,
    ring: Arc<LiveStemRing>,
    handle: Option<JoinHandle<()>>,
}

impl LiveSeparator {
    /// Build a handle around an already-spawned producer.
    ///
    /// Used by the StemgenRT producer, which shares the same stop/join and
    /// ring-coverage lifecycle as the HTDemucs live path.
    pub(crate) fn from_parts(
        stop: Arc<AtomicBool>,
        ring: Arc<LiveStemRing>,
        handle: JoinHandle<()>,
    ) -> Self {
        Self {
            stop,
            ring,
            handle: Some(handle),
        }
    }

    /// Ask the producer to stop; coverage is cleared immediately so the deck
    /// falls back to the main mix.
    pub fn stop(&self) {
        self.stop.store(true, Ordering::Release);
        self.ring.close();
    }

    /// Ask the producer to stop and wait for it to exit (shutdown / tests).
    pub fn join(mut self) {
        self.stop();
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for LiveSeparator {
    fn drop(&mut self) {
        // Never join here: the host drops this while holding locks and the
        // producer may be inside a multi-second ONNX run. The stop flag is
        // checked between chunks and the thread detaches.
        self.stop();
    }
}

/// Start separating `audio` into `ring` on a background thread.
///
/// The model must already be on disk (`weights`); this never downloads.
/// Returns an error for non-stereo input, a zero rate, or a ring rate that is
/// not the model rate.
pub fn start_live_separator(
    weights: PathBuf,
    audio: Arc<LoadedAudio>,
    ring: Arc<LiveStemRing>,
) -> Result<LiveSeparator> {
    if audio.channels != 2 {
        bail!(
            "live stems require interleaved stereo (got {} channels)",
            audio.channels
        );
    }
    if audio.sample_rate == 0 {
        bail!("live stems require a non-zero sample rate");
    }
    if ring.rate() != SAMPLE_RATE {
        bail!(
            "live stem ring rate must be {SAMPLE_RATE} (got {})",
            ring.rate()
        );
    }

    let stop = Arc::new(AtomicBool::new(false));
    let thread_stop = Arc::clone(&stop);
    let thread_ring = Arc::clone(&ring);
    let handle = thread::Builder::new()
        .name("stems-live".into())
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
                warn!(error = %message, "live stems: producer panicked");
                thread_ring.close();
            }
        })
        .map_err(|e| anyhow::anyhow!("spawn live stem thread: {e}"))?;

    Ok(LiveSeparator {
        stop,
        ring,
        handle: Some(handle),
    })
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
                warn!(error = %err, "live stems: resample failed");
                ring.close();
                return;
            }
        }
    };

    let frames = (pcm.len() / 2) as u64;
    if frames == 0 {
        ring.close();
        return;
    }

    let weights_vec = infer::chunk_weights();
    let mut chunk = vec![0.0f32; SEGMENT_SAMPLES * 2];
    let mut window: [Vec<f32>; SOURCES] = std::array::from_fn(|_| vec![0.0f32; CHUNK_STRIDE * 2]);
    let mut prev: Option<[Vec<f32>; SOURCES]> = None;
    let mut origin = 0u64;
    let mut next_window = 0usize;
    let mut played = false;
    let mut ep_logged = false;

    // Never let a published window overwrite frames at or ahead of the reader.
    let max_ahead = max_ahead_frames(ring.capacity_frames());
    debug_assert!(max_ahead > 0, "ring too small for one stride");

    let mut separate_at = |start: u64| -> Result<[Vec<f32>; SOURCES]> {
        let valid = ((frames - start) as usize).min(SEGMENT_SAMPLES);
        infer::fill_chunk_reflect(pcm, frames as usize, start as usize, valid, &mut chunk);
        session::with_session(weights, |session, ep, input_name, output_name| {
            if !ep_logged {
                info!(ep, "live stems: ORT session ready");
                ep_logged = true;
            }
            infer::run_chunk(session, &chunk, input_name, output_name)
        })
    };

    loop {
        if stop.load(Ordering::Acquire) || ring.is_closed() {
            break;
        }

        let playhead = ring.playhead().min(frames);
        let mut reset = !played;
        if let Some((start, end)) = ring.coverage() {
            let forward_seek = playhead > end.saturating_add(CHUNK_STRIDE as u64);
            reset = reset || playhead < start || forward_seek;
        }
        if reset {
            // Anchor epochs to the offline chunk grid so a seek produces the
            // same windows (and overlap) a cached separation would have.
            let grid = (playhead / CHUNK_STRIDE as u64) * CHUNK_STRIDE as u64;
            origin = grid;
            next_window = 0;
            prev = None;
            ring.reset(grid);
            played = true;
            if grid >= CHUNK_STRIDE as u64 && grid < frames {
                // Context chunk for the first window's overlap fade.
                match separate_at(grid - CHUNK_STRIDE as u64) {
                    Ok(context) => prev = Some(context),
                    Err(err) => {
                        warn!(error = %err, "live stems: separation failed");
                        ring.close();
                        break;
                    }
                }
            }
            info!(frame = grid, "live stems: epoch start");
        } else if let Some((_, end)) = ring.coverage() {
            if end.saturating_sub(playhead) >= max_ahead {
                thread::sleep(POLL);
                continue;
            }
        }

        let window_start = origin.saturating_add((next_window as u64) * CHUNK_STRIDE as u64);
        if window_start >= frames {
            // Whole track covered for this epoch; park until the deck seeks.
            thread::sleep(POLL);
            continue;
        }

        let window_len = ((frames - window_start) as usize).min(CHUNK_STRIDE);
        let current = match separate_at(window_start) {
            Ok(current) => current,
            Err(err) => {
                warn!(error = %err, "live stems: separation failed");
                ring.close();
                break;
            }
        };

        if ring.is_closed() || stop.load(Ordering::Acquire) {
            break;
        }
        infer::finalize_window(
            &weights_vec,
            prev.as_ref(),
            &current,
            window_len,
            &mut window,
        );
        ring.publish(
            window_start,
            window_len,
            &[
                window[0].as_slice(),
                window[1].as_slice(),
                window[2].as_slice(),
                window[3].as_slice(),
            ],
        );
        prev = Some(current);
        next_window += 1;
    }

    info!("live stems: producer stopped");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pacing_never_crosses_capacity() {
        for capacity in [1 << 19, 1 << 20, 1 << 21] {
            let ahead = max_ahead_frames(capacity);
            assert!(ahead > 0, "capacity {capacity} stalls");
            assert!(
                ahead + CHUNK_STRIDE as u64 + 1 < capacity as u64,
                "capacity {capacity}: window wraps onto the reader"
            );
        }
    }
}
