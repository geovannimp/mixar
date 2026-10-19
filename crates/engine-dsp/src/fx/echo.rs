//! Beat-synced echo (DJM-style `BEAT ◀ ▶` delay).

use audio_core::Sample;

use super::{Fx, DEFAULT_FX_BPM, FX_PARAM_COUNT};

/// Beat fractions for the TIME knob, in beats (DJM-style `BEAT ◀ ▶` steps).
pub const ECHO_DIVISIONS: [f32; 7] = [
    1.0 / 16.0,
    1.0 / 8.0,
    1.0 / 4.0,
    1.0 / 2.0,
    3.0 / 4.0,
    1.0,
    2.0,
];
/// Default TIME position: `1/2` beat (`ECHO_DIVISIONS[3]`, norm `0.5`).
pub const ECHO_DEFAULT_DIVISION: usize = 3;
/// Longest delay the line holds (`2` beats at `ECHO_MIN_SYNC_BPM`).
pub const ECHO_MAX_DELAY_MS: f32 = 2_000.0;
/// Slowest master tempo the sync math supports; delay is clamped to the line length below it.
pub const ECHO_MIN_SYNC_BPM: f32 = 60.0;
/// Feedback ceiling (norm `1.0` ⇒ `0.9`).
pub const ECHO_MAX_FEEDBACK: f32 = 0.9;
/// Default FEEDBACK knob (norm `0.5` ⇒ `0.45`).
pub const ECHO_DEFAULT_FEEDBACK: f32 = 0.5;
/// Read-tap crossfade length used when the target delay moves a lot.
const XFADE_MS: f32 = 20.0;
/// Delay moves smaller than this (frames) move the live tap directly.
const SLEW_EPS_FRAMES: f32 = 16.0;

/// Delay line with two read taps; the live tap chases the target so beat-fraction
/// changes crossfade instead of clicking.
#[derive(Debug)]
pub struct Echo {
    sample_rate: f32,
    /// Interleaved stereo delay line (`max_frames * 2`), allocated in `new`.
    buf: Vec<Sample>,
    max_frames: usize,
    write: usize,
    /// Tap the live read is leaving.
    tap_a: f32,
    /// Tap the live read is moving to.
    tap_b: f32,
    /// Crossfade position from `tap_a` (0.0) to `tap_b` (1.0).
    blend: f32,
    /// Blend step per frame.
    xfade_inc: f32,
    feedback: f32,
    bpm: f32,
    division: usize,
}

impl Echo {
    pub fn new(sample_rate: u32) -> Self {
        let sample_rate_f = sample_rate.max(1) as f32;
        let max_frames = (sample_rate_f * ECHO_MAX_DELAY_MS / 1000.0).ceil() as usize;
        let mut echo = Self {
            sample_rate: sample_rate_f,
            buf: vec![0.0; max_frames.max(2) * 2],
            max_frames: max_frames.max(2),
            write: 0,
            tap_a: 0.0,
            tap_b: 0.0,
            blend: 1.0,
            xfade_inc: 1000.0 / (XFADE_MS * sample_rate_f),
            feedback: ECHO_DEFAULT_FEEDBACK * ECHO_MAX_FEEDBACK,
            bpm: DEFAULT_FX_BPM,
            division: ECHO_DEFAULT_DIVISION,
        };
        let target = echo.target_frames();
        echo.tap_a = target;
        echo.tap_b = target;
        echo
    }

    /// Delay length of the selected beat fraction at the current sync tempo.
    fn target_frames(&self) -> f32 {
        let beat_ms = 60_000.0 / self.bpm.max(ECHO_MIN_SYNC_BPM);
        let frames = beat_ms * ECHO_DIVISIONS[self.division] * self.sample_rate / 1000.0;
        // `max_frames` is floored at 2, so the upper bound can reach 0.0 for a
        // degenerate sample rate; `f32::clamp` panics when min > max.
        frames.clamp(1.0, (self.max_frames as f32 - 2.0).max(1.0))
    }

    /// Live read offset, between the two taps during a crossfade.
    fn current_offset(&self) -> f32 {
        self.tap_a + (self.tap_b - self.tap_a) * self.blend
    }

    /// Move the live tap to `target_frames()`, crossfading when the jump is large.
    fn retarget(&mut self) {
        let target = self.target_frames();
        if (target - self.tap_b).abs() > SLEW_EPS_FRAMES {
            self.tap_a = self.current_offset();
            self.tap_b = target;
            self.blend = 0.0;
        } else {
            self.tap_b = target;
        }
    }

    /// Interpolated stereo read at `offset_frames` behind the write head.
    fn read(&self, offset_frames: f32) -> (f32, f32) {
        let max_frames = self.max_frames;
        let mut pos = self.write as f32 - offset_frames;
        while pos < 0.0 {
            pos += max_frames as f32;
        }
        while pos >= max_frames as f32 {
            pos -= max_frames as f32;
        }
        let i0 = pos.floor() as usize % max_frames;
        let i1 = (i0 + 1) % max_frames;
        let frac = pos - pos.floor();
        let left = self.buf[i0 * 2] * (1.0 - frac) + self.buf[i1 * 2] * frac;
        let right = self.buf[i0 * 2 + 1] * (1.0 - frac) + self.buf[i1 * 2 + 1] * frac;
        (left, right)
    }

    fn write_frame(&mut self, left: f32, right: f32) {
        self.buf[self.write * 2] = left;
        self.buf[self.write * 2 + 1] = right;
        self.write = (self.write + 1) % self.max_frames;
    }
}

impl Fx for Echo {
    fn param_count(&self) -> usize {
        2
    }

    fn default_params(&self) -> [f32; FX_PARAM_COUNT] {
        [
            ECHO_DEFAULT_DIVISION as f32 / (ECHO_DIVISIONS.len() - 1) as f32,
            ECHO_DEFAULT_FEEDBACK,
            0.0,
        ]
    }

    fn set_param(&mut self, index: usize, value: f32) {
        match index {
            0 => {
                self.division = (value * (ECHO_DIVISIONS.len() - 1) as f32).round() as usize;
                self.division = self.division.min(ECHO_DIVISIONS.len() - 1);
                self.retarget();
            }
            1 => self.feedback = value * ECHO_MAX_FEEDBACK,
            _ => debug_assert!(false, "param index out of range"),
        }
    }

    fn set_tempo(&mut self, bpm: f32) {
        if bpm.is_finite() && bpm > 0.0 {
            self.bpm = bpm;
            self.retarget();
        }
    }

    fn reset(&mut self) {
        self.buf.fill(0.0);
        self.write = 0;
        self.blend = 1.0;
        let target = self.target_frames();
        self.tap_a = target;
        self.tap_b = target;
    }

    fn process_frame(&mut self, left: f32, right: f32) -> (f32, f32) {
        if self.blend < 1.0 {
            self.blend = (self.blend + self.xfade_inc).min(1.0);
        }
        let (a_l, a_r) = self.read(self.tap_a);
        let (b_l, b_r) = self.read(self.tap_b);
        let delayed = (
            a_l + (b_l - a_l) * self.blend,
            a_r + (b_r - a_r) * self.blend,
        );
        self.write_frame(
            left + delayed.0 * self.feedback,
            right + delayed.1 * self.feedback,
        );
        delayed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_RATE: u32 = 48_000;
    /// 1/2 beat at 120 BPM.
    const HALF_BEAT_FRAMES: usize = 12_000;

    fn render(echo: &mut Echo, frames: usize) -> Vec<f32> {
        render_input(echo, frames, |frame| if frame == 0 { 1.0 } else { 0.0 })
    }

    fn render_silence(echo: &mut Echo, frames: usize) -> Vec<f32> {
        render_input(echo, frames, |_| 0.0)
    }

    fn render_input(
        echo: &mut Echo,
        frames: usize,
        mut input_at: impl FnMut(usize) -> f32,
    ) -> Vec<f32> {
        let mut out = Vec::with_capacity(frames * 2);
        for frame in 0..frames {
            let input = input_at(frame);
            let (left, right) = echo.process_frame(input, input);
            out.push(left);
            out.push(right);
        }
        out
    }

    #[test]
    fn half_beat_at_120_bpm_is_250_ms() {
        let mut echo = Echo::new(SAMPLE_RATE);
        echo.set_tempo(120.0);
        let out = render(&mut echo, 20_000);

        assert_eq!(out[0], 0.0, "echo output is wet only");
        assert!(
            out[HALF_BEAT_FRAMES * 2].abs() > 0.9,
            "impulse must return after 250 ms, got {}",
            out[HALF_BEAT_FRAMES * 2]
        );
    }

    #[test]
    fn feedback_repeats_decay() {
        let mut echo = Echo::new(SAMPLE_RATE);
        echo.set_tempo(120.0);
        // 1/4 beat at 120 BPM.
        echo.set_param(0, 2.0 / (ECHO_DIVISIONS.len() - 1) as f32);
        echo.set_param(1, 1.0);
        let out = render(&mut echo, 20_000);

        let at_quarter = out[6_000 * 2].abs();
        let at_half = out[12_000 * 2].abs();
        let at_three_quarters = out[18_000 * 2].abs();
        assert!(
            (at_quarter - 1.0).abs() < 1e-2,
            "first repeat: {at_quarter}"
        );
        assert!((at_half - 0.9).abs() < 1e-2, "second repeat: {at_half}");
        assert!(
            (at_three_quarters - 0.81).abs() < 1e-2,
            "third repeat: {at_three_quarters}"
        );
    }

    #[test]
    fn division_knob_snaps_to_table() {
        let mut echo = Echo::new(SAMPLE_RATE);
        echo.set_tempo(120.0);

        echo.set_param(0, 0.0);
        let out = render(&mut echo, 4_000);
        // 1/16 beat at 120 BPM = 31.25 ms = 1500 frames.
        assert!(out[0] == 0.0);
        assert!(out[1_500 * 2].abs() > 0.9, "1/16 beat repeat missing");

        echo.set_param(0, 1.0);
        let out = render(&mut echo, 60_000);
        // 2/1 beats at 120 BPM = 1 s = 48000 frames after this impulse.
        assert!(
            out[48_000 * 2].abs() > 0.9,
            "2/1 beat repeat missing: {}",
            out[48_000 * 2]
        );
    }

    #[test]
    fn time_change_crossfades() {
        let mut echo = Echo::new(SAMPLE_RATE);
        echo.set_tempo(120.0);
        render(&mut echo, 500);

        // 1/2 beat (12000 frames) → 1/16 beat (1500 frames): the live tap crossfades.
        echo.set_param(0, 0.0);
        let crossfade = (XFADE_MS * SAMPLE_RATE as f32 / 1000.0).round() as usize;
        let out = render_silence(&mut echo, 4_000);

        let peak = out.iter().fold(0.0f32, |acc, s| acc.max(s.abs()));
        assert!(peak <= 1.2, "crossfade must not blow up: {peak}");
        let onset = out
            .chunks(2)
            .position(|frame| frame[0].abs() > 0.9)
            .expect("repeat must arrive");
        let expected = 1_500 - 500;
        assert!(
            onset.abs_diff(expected) <= crossfade,
            "repeat at {onset} frames, expected {expected} ± {crossfade}"
        );
    }

    #[test]
    fn degenerate_sample_rate_does_not_panic() {
        // sample_rate <= 1 gives the shortest line the constructor allows, which
        // used to make `target_frames` clamp with min > max.
        for rate in [0, 1] {
            let mut echo = Echo::new(rate);
            echo.set_tempo(120.0);
            echo.set_param(0, 1.0);
            let (left, right) = echo.process_frame(0.5, -0.5);
            assert!(left.is_finite() && right.is_finite());
            echo.reset();
        }
    }

    #[test]
    fn reset_clears_tail() {
        let mut echo = Echo::new(SAMPLE_RATE);
        let _ = render(&mut echo, 1_000);
        echo.reset();
        let out = render(&mut echo, 8_000);
        assert!(out.iter().all(|s| *s == 0.0), "reset must clear the line");
    }
}
