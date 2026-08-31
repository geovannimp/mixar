//! Realtime time-stretch trait + timestretch engine for Mixar key lock.

mod engine;

pub use engine::TimestretchStretcher;

use audio_core::Sample;

/// Realtime time stretcher (tempo change with pitch held for key lock).
pub trait TimeStretcher: Send {
    /// Engine / processing sample rate.
    fn sample_rate(&self) -> u32;

    /// Set playback tempo rate (`1.0` = original; `>1` = faster). Pitch stays locked.
    fn set_tempo_rate(&mut self, rate: f64);

    /// Frames of silence / pad before real audio (may be zero).
    fn preferred_start_pad(&self) -> usize;

    /// Output latency in frames (for playhead compensation).
    fn start_delay(&self) -> usize;

    /// Clear stretcher history (seek / load / key-lock toggle).
    fn reset(&mut self);

    /// Pull `out_frames` of interleaved stereo into `output`.
    ///
    /// `feed` supplies up to `need` interleaved stereo frames from the source
    /// (or silence). Returns frames actually written into the provided buffer.
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
    /// Source frames passed through `feed`.
    pub source_frames_fed: usize,
    /// Output frames written.
    pub out_frames: usize,
}

/// Create the default realtime stretcher ([`timestretch`] WideKeylock profile).
pub fn create_stretcher(
    sample_rate: u32,
    max_process_frames: usize,
) -> anyhow::Result<Box<dyn TimeStretcher>> {
    Ok(Box::new(TimestretchStretcher::new(
        sample_rate,
        max_process_frames,
    )?))
}
