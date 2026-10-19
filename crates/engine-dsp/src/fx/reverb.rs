//! Freeverb-style reverb (Schroeder combs + allpasses), stereo-spread.

use audio_core::Sample;

use super::{Fx, FX_PARAM_COUNT};

const COMB_TUNING: [usize; 8] = [1116, 1188, 1277, 1356, 1422, 1491, 1557, 1617];
const ALLPASS_TUNING: [usize; 4] = [556, 441, 341, 225];
const STEREO_SPREAD: usize = 23;
const FIXED_GAIN: f32 = 0.015;
const SCALE_WET: f32 = 3.0;
const ALLPASS_FEEDBACK: f32 = 0.5;
const SIZE_MIN_FEEDBACK: f32 = 0.70;
const SIZE_SPAN_FEEDBACK: f32 = 0.28;
const DAMP_SPAN: f32 = 0.4;
const DEFAULT_SIZE: f32 = 0.5;
const DEFAULT_DAMPING: f32 = 0.5;

#[derive(Debug)]
struct Comb {
    buf: Vec<Sample>,
    idx: usize,
    store: f32,
}

impl Comb {
    fn new(len: usize) -> Self {
        Self {
            buf: vec![0.0; len.max(1)],
            idx: 0,
            store: 0.0,
        }
    }

    fn process(&mut self, input: f32, feedback: f32, damp: f32) -> f32 {
        let out = self.buf[self.idx];
        self.store = out * (1.0 - damp) + self.store * damp;
        self.buf[self.idx] = input + self.store * feedback;
        self.idx = (self.idx + 1) % self.buf.len();
        out
    }

    fn reset(&mut self) {
        self.buf.fill(0.0);
        self.idx = 0;
        self.store = 0.0;
    }
}

#[derive(Debug)]
struct AllPass {
    buf: Vec<Sample>,
    idx: usize,
}

impl AllPass {
    fn new(len: usize) -> Self {
        Self {
            buf: vec![0.0; len.max(1)],
            idx: 0,
        }
    }

    fn process(&mut self, input: f32) -> f32 {
        let bufout = self.buf[self.idx];
        let out = -input + bufout;
        self.buf[self.idx] = input + bufout * ALLPASS_FEEDBACK;
        self.idx = (self.idx + 1) % self.buf.len();
        out
    }

    fn reset(&mut self) {
        self.buf.fill(0.0);
        self.idx = 0;
    }
}

/// Stereo reverb with SIZE and DAMPING knobs (not beat-synced).
#[derive(Debug)]
pub struct Reverb {
    combs_l: [Comb; 8],
    combs_r: [Comb; 8],
    allpass_l: [AllPass; 4],
    allpass_r: [AllPass; 4],
    feedback: f32,
    damp: f32,
}

impl Reverb {
    pub fn new(sample_rate: u32) -> Self {
        let scale = sample_rate.max(1) as f32 / 44_100.0;
        let comb_len = |tuning: usize, spread: usize| {
            (((tuning + spread) as f32) * scale).round().max(1.0) as usize
        };
        let allpass_len = |tuning: usize, spread: usize| {
            (((tuning + spread) as f32) * scale).round().max(1.0) as usize
        };

        Self {
            combs_l: std::array::from_fn(|i| Comb::new(comb_len(COMB_TUNING[i], 0))),
            combs_r: std::array::from_fn(|i| Comb::new(comb_len(COMB_TUNING[i], STEREO_SPREAD))),
            allpass_l: std::array::from_fn(|i| AllPass::new(allpass_len(ALLPASS_TUNING[i], 0))),
            allpass_r: std::array::from_fn(|i| {
                AllPass::new(allpass_len(ALLPASS_TUNING[i], STEREO_SPREAD))
            }),
            feedback: SIZE_MIN_FEEDBACK + SIZE_SPAN_FEEDBACK * DEFAULT_SIZE,
            damp: DAMP_SPAN * DEFAULT_DAMPING,
        }
    }
}

impl Fx for Reverb {
    fn param_count(&self) -> usize {
        2
    }

    fn default_params(&self) -> [f32; FX_PARAM_COUNT] {
        [DEFAULT_SIZE, DEFAULT_DAMPING, 0.0]
    }

    fn set_param(&mut self, index: usize, value: f32) {
        match index {
            0 => self.feedback = SIZE_MIN_FEEDBACK + SIZE_SPAN_FEEDBACK * value,
            1 => self.damp = DAMP_SPAN * value,
            _ => debug_assert!(false, "param index out of range"),
        }
    }

    fn reset(&mut self) {
        for comb in self.combs_l.iter_mut().chain(self.combs_r.iter_mut()) {
            comb.reset();
        }
        for allpass in self.allpass_l.iter_mut().chain(self.allpass_r.iter_mut()) {
            allpass.reset();
        }
    }

    fn process_frame(&mut self, left: f32, right: f32) -> (f32, f32) {
        let input = (left + right) * 0.5 * FIXED_GAIN;
        let feedback = self.feedback;
        let damp = self.damp;

        let mut out_l = 0.0;
        for comb in &mut self.combs_l {
            out_l += comb.process(input, feedback, damp);
        }
        for allpass in &mut self.allpass_l {
            out_l = allpass.process(out_l);
        }

        let mut out_r = 0.0;
        for comb in &mut self.combs_r {
            out_r += comb.process(input, feedback, damp);
        }
        for allpass in &mut self.allpass_r {
            out_r = allpass.process(out_r);
        }

        (out_l * SCALE_WET, out_r * SCALE_WET)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_RATE: u32 = 48_000;
    const FRAMES: usize = 48_000;

    fn render(reverb: &mut Reverb, input_at: impl FnMut(usize) -> (f32, f32)) -> Vec<f32> {
        let mut input_at = input_at;
        let mut out = Vec::with_capacity(FRAMES * 2);
        for frame in 0..FRAMES {
            let (left, right) = input_at(frame);
            let (wet_l, wet_r) = reverb.process_frame(left, right);
            out.push(wet_l);
            out.push(wet_r);
        }
        out
    }

    fn energy(out: &[f32], from: usize, to: usize) -> f32 {
        out[from * 2..(to * 2).min(out.len())]
            .iter()
            .map(|s| s * s)
            .sum()
    }

    #[test]
    fn impulse_produces_a_decaying_tail() {
        let mut reverb = Reverb::new(SAMPLE_RATE);
        let out = render(&mut reverb, |frame| {
            if frame == 0 {
                (1.0, 1.0)
            } else {
                (0.0, 0.0)
            }
        });

        let onset = out
            .chunks(2)
            .position(|frame| frame[0].abs() > 0.0)
            .expect("impulse must reach the output");
        assert!(onset >= 1_000, "onset at {onset} frames is too early");

        let early = energy(&out, 1_500, 5_500);
        let late = energy(&out, 44_000, 48_000);
        assert!(early > 0.0, "reverb tail must be audible");
        assert!(late < early, "tail must decay: {late} !< {early}");

        let nonzero_frames = out
            .chunks(2)
            .take(30_000)
            .filter(|frame| frame[0] != 0.0)
            .count();
        assert!(
            nonzero_frames > 5_000,
            "tail must ring for a while, got {nonzero_frames} frames"
        );
    }

    #[test]
    fn full_scale_input_stays_bounded() {
        let mut reverb = Reverb::new(SAMPLE_RATE);
        let out = render(&mut reverb, |frame| {
            let value = if frame % 2 == 0 { 1.0 } else { -1.0 };
            (value, value)
        });
        let peak = out.iter().fold(0.0f32, |acc, s| acc.max(s.abs()));
        assert!(peak < 4.0, "output must stay bounded, peak {peak}");
    }

    #[test]
    fn reset_silences_tail() {
        let mut reverb = Reverb::new(SAMPLE_RATE);
        let mut out = Vec::with_capacity(FRAMES * 2);
        for frame in 0..1_000 {
            let input = if frame == 0 { 1.0 } else { 0.0 };
            let (wet_l, wet_r) = reverb.process_frame(input, input);
            out.push(wet_l);
            out.push(wet_r);
        }
        reverb.reset();
        for _ in 0..8_000 {
            let (wet_l, wet_r) = reverb.process_frame(0.0, 0.0);
            out.push(wet_l);
            out.push(wet_r);
        }
        assert!(
            out[1_000 * 2..].iter().all(|s| *s == 0.0),
            "reset must silence the tail"
        );
    }
}
