//! Rubber Band realtime stretcher (key lock: pitch scale fixed at 1.0).

use crate::ffi::{
    self, RubberBandState, RUBBERBAND_OPTION_CHANNELS_TOGETHER, RUBBERBAND_OPTION_ENGINE_FINER,
    RUBBERBAND_OPTION_PITCH_HIGH_CONSISTENCY, RUBBERBAND_OPTION_PROCESS_REAL_TIME,
    RUBBERBAND_OPTION_THREADING_NEVER,
};
use crate::{StretchPullStats, TimeStretcher};
use anyhow::{anyhow, Result};
use audio_core::Sample;

/// Rubber Band realtime wrapper.
pub struct RubberBandStretcher {
    state: RubberBandState,
    sample_rate: u32,
    /// Remaining start-pad silence frames to feed before real audio.
    pad_remaining: usize,
    /// Deinterleave scratch (L then R planar).
    planar_l: Vec<f32>,
    planar_r: Vec<f32>,
    /// Interleaved feed scratch for `feed` callback.
    feed_scratch: Vec<Sample>,
    /// Silence buffer for pad / shortfall.
    silence: Vec<Sample>,
}

impl RubberBandStretcher {
    /// Create a realtime R3 stretcher at `sample_rate` (stereo).
    pub fn new(sample_rate: u32, max_process_frames: usize) -> Result<Self> {
        if sample_rate == 0 {
            return Err(anyhow!("sample_rate must be > 0"));
        }
        let options = RUBBERBAND_OPTION_PROCESS_REAL_TIME
            | RUBBERBAND_OPTION_THREADING_NEVER
            | RUBBERBAND_OPTION_CHANNELS_TOGETHER
            | RUBBERBAND_OPTION_ENGINE_FINER
            | RUBBERBAND_OPTION_PITCH_HIGH_CONSISTENCY;

        // SAFETY: Rubber Band C ctor; null means allocation failure.
        let state = unsafe { ffi::rubberband_new(sample_rate, 2, options, 1.0, 1.0) };
        if state.is_null() {
            return Err(anyhow!("rubberband_new failed"));
        }

        let max_frames = max_process_frames.max(512);
        unsafe {
            ffi::rubberband_set_max_process_size(state, max_frames as u32);
            ffi::rubberband_set_pitch_scale(state, 1.0);
        }

        let pad = unsafe { ffi::rubberband_get_preferred_start_pad(state) as usize };

        Ok(Self {
            state,
            sample_rate,
            pad_remaining: pad,
            planar_l: vec![0.0; max_frames],
            planar_r: vec![0.0; max_frames],
            feed_scratch: vec![0.0; max_frames * 2],
            silence: vec![0.0; max_frames * 2],
        })
    }

    fn process_planar(&mut self, frames: usize) {
        if frames == 0 {
            return;
        }
        let input = [self.planar_l.as_ptr(), self.planar_r.as_ptr()];
        unsafe {
            ffi::rubberband_process(self.state, input.as_ptr(), frames as u32, 0);
        }
    }

    fn feed_pad_or_source(
        &mut self,
        need: usize,
        feed: &mut dyn FnMut(usize, &mut [Sample]) -> usize,
    ) -> usize {
        if need == 0 {
            return 0;
        }
        if need > self.planar_l.len() {
            self.planar_l.resize(need, 0.0);
            self.planar_r.resize(need, 0.0);
            self.feed_scratch.resize(need * 2, 0.0);
            self.silence.resize(need * 2, 0.0);
        }

        let mut source_fed = 0usize;
        let mut filled = 0usize;

        while filled < need {
            let remaining = need - filled;
            if self.pad_remaining > 0 {
                let n = remaining.min(self.pad_remaining);
                for i in 0..n {
                    self.planar_l[filled + i] = 0.0;
                    self.planar_r[filled + i] = 0.0;
                }
                self.pad_remaining -= n;
                filled += n;
                continue;
            }

            let scratch = &mut self.feed_scratch[..remaining * 2];
            scratch.fill(0.0);
            let got = feed(remaining, scratch);
            source_fed += got;
            for i in 0..remaining {
                let base = i * 2;
                if i < got {
                    self.planar_l[filled + i] = scratch[base];
                    self.planar_r[filled + i] = scratch[base + 1];
                } else {
                    self.planar_l[filled + i] = 0.0;
                    self.planar_r[filled + i] = 0.0;
                }
            }
            filled += remaining;
        }

        self.process_planar(need);
        source_fed
    }
}

impl TimeStretcher for RubberBandStretcher {
    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn set_time_ratio(&mut self, ratio: f64) {
        let ratio = if ratio.is_finite() && ratio > 0.0 {
            ratio.clamp(0.05, 10.0)
        } else {
            1.0
        };
        unsafe {
            ffi::rubberband_set_time_ratio(self.state, ratio);
            ffi::rubberband_set_pitch_scale(self.state, 1.0);
        }
    }

    fn preferred_start_pad(&self) -> usize {
        unsafe { ffi::rubberband_get_preferred_start_pad(self.state) as usize }
    }

    fn start_delay(&self) -> usize {
        unsafe { ffi::rubberband_get_start_delay(self.state) as usize }
    }

    fn reset(&mut self) {
        unsafe {
            ffi::rubberband_reset(self.state);
        }
        self.pad_remaining = self.preferred_start_pad();
    }

    fn pull_interleaved(
        &mut self,
        out_frames: usize,
        output: &mut [Sample],
        feed: &mut dyn FnMut(usize, &mut [Sample]) -> usize,
    ) -> StretchPullStats {
        let need_samples = out_frames * 2;
        if output.len() < need_samples || out_frames == 0 {
            return StretchPullStats::default();
        }

        let mut source_frames_fed = 0usize;
        let mut written = 0usize;

        // Bound iterations so a stuck stretcher cannot hang the audio thread.
        let mut spins = 0usize;
        const MAX_SPINS: usize = 64;

        while written < out_frames && spins < MAX_SPINS {
            spins += 1;
            let available = unsafe { ffi::rubberband_available(self.state) };
            if available < 0 {
                break;
            }
            let available = available as usize;
            if available > 0 {
                let take = (out_frames - written)
                    .min(available)
                    .min(self.planar_l.len());
                if take > self.planar_l.len() {
                    self.planar_l.resize(take, 0.0);
                    self.planar_r.resize(take, 0.0);
                }
                let mut out_ptrs = [self.planar_l.as_mut_ptr(), self.planar_r.as_mut_ptr()];
                let got = unsafe {
                    ffi::rubberband_retrieve(self.state, out_ptrs.as_mut_ptr(), take as u32)
                } as usize;
                for i in 0..got {
                    let o = (written + i) * 2;
                    output[o] = self.planar_l[i];
                    output[o + 1] = self.planar_r[i];
                }
                written += got;
                continue;
            }

            let required = unsafe { ffi::rubberband_get_samples_required(self.state) as usize };
            let chunk = required.max(1).min(self.planar_l.len().max(1));
            source_frames_fed += self.feed_pad_or_source(chunk, feed);
        }

        if written < out_frames {
            output[written * 2..out_frames * 2].fill(0.0);
        }

        StretchPullStats {
            source_frames_fed,
            out_frames: written,
        }
    }
}

impl Drop for RubberBandStretcher {
    fn drop(&mut self) {
        if !self.state.is_null() {
            unsafe {
                ffi::rubberband_delete(self.state);
            }
            self.state = std::ptr::null_mut();
        }
    }
}

// RubberBandState is a raw pointer; Send is required for deck use on the producer thread.
unsafe impl Send for RubberBandStretcher {}
