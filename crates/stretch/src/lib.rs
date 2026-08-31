//! Realtime time-stretch trait + Rubber Band implementation for Mixar key lock.

mod ffi;
mod rubberband;

pub use rubberband::RubberBandStretcher;

use audio_core::Sample;

/// Realtime time stretcher (tempo change with pitch scale held at 1.0 for key lock).
pub trait TimeStretcher: Send {
    /// Engine / processing sample rate.
    fn sample_rate(&self) -> u32;

    /// Set time ratio (`1.0` = original tempo). Pitch scale stays `1.0`.
    fn set_time_ratio(&mut self, ratio: f64);

    /// Frames of silence to feed before real audio (Rubber Band start pad).
    fn preferred_start_pad(&self) -> usize;

    /// Output latency in frames after start pad (for playhead compensation).
    fn start_delay(&self) -> usize;

    /// Clear stretcher history (seek / load / key-lock toggle).
    fn reset(&mut self);

    /// Pull `out_frames` of interleaved stereo into `output`.
    ///
    /// `feed` supplies up to `need` interleaved stereo frames from the source
    /// (or silence). Returns source frames consumed via `feed` plus any pad.
    ///
    /// `feed` return value is frames actually written into the provided buffer.
    fn pull_interleaved(
        &mut self,
        out_frames: usize,
        output: &mut [Sample],
        feed: &mut dyn FnMut(usize, &mut [Sample]) -> usize,
    ) -> StretchPullStats;
}

/// Accounting for one stretch pull.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StretchPullStats {
    /// Source frames passed through `feed` (excludes synthetic start pad).
    pub source_frames_fed: usize,
    /// Output frames written.
    pub out_frames: usize,
}

/// Create the default realtime stretcher (Rubber Band R3 when available).
pub fn create_stretcher(
    sample_rate: u32,
    max_process_frames: usize,
) -> anyhow::Result<Box<dyn TimeStretcher>> {
    Ok(Box::new(RubberBandStretcher::new(
        sample_rate,
        max_process_frames,
    )?))
}
