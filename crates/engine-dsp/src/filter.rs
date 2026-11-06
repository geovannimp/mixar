//! DJ-style one-knob filter (LP left, HP right, center = bypass).

use audio_core::Sample;

use crate::eq::{clamp_gain_db, EQ_MAX_DB};

const LP_CUTOFF_HZ: f32 = 180.0;
const HP_CUTOFF_HZ: f32 = 1_800.0;

/// Frames between coefficient recomputes (~0.33 ms at 48 kHz): the cutoff
/// glides in steps too short to hear, without a trig call per sample.
const COEFF_UPDATE_FRAMES: usize = 16;

/// Time a full-range knob jump takes to glide in. Long enough to stay
/// click-free, short enough that the knob still feels attached.
const SWEEP_SECS: f32 = 0.02;

#[derive(Debug, Clone, Copy)]
struct BiquadCoeffs {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
}

#[derive(Debug, Clone, Copy, Default)]
struct BiquadState {
    z1: f32,
    z2: f32,
}

#[derive(Debug, Clone)]
struct Biquad {
    coeffs: BiquadCoeffs,
    left: BiquadState,
    right: BiquadState,
}

impl Biquad {
    fn new(coeffs: BiquadCoeffs) -> Self {
        Self {
            coeffs,
            left: BiquadState::default(),
            right: BiquadState::default(),
        }
    }

    /// Swap coefficients in place: the delay line is the filter's history and
    /// must survive, or every knob step is a transient.
    fn set_coeffs(&mut self, coeffs: BiquadCoeffs) {
        self.coeffs = coeffs;
    }

    fn reset(&mut self) {
        self.left = BiquadState::default();
        self.right = BiquadState::default();
    }

    fn process_stereo(&mut self, left: f32, right: f32) -> (f32, f32) {
        let coeffs = self.coeffs;
        let l = process_sample(coeffs, left, &mut self.left);
        let r = process_sample(coeffs, right, &mut self.right);
        (l, r)
    }
}

fn process_sample(coeffs: BiquadCoeffs, input: f32, state: &mut BiquadState) -> f32 {
    let x = input;
    let y = coeffs.b0 * x + state.z1;
    state.z1 = coeffs.b1 * x - coeffs.a1 * y + state.z2;
    state.z2 = coeffs.b2 * x - coeffs.a2 * y;
    y
}

fn identity_coeffs() -> BiquadCoeffs {
    BiquadCoeffs {
        b0: 1.0,
        b1: 0.0,
        b2: 0.0,
        a1: 0.0,
        a2: 0.0,
    }
}

fn lowpass_coeffs(sample_rate: f32, frequency: f32) -> BiquadCoeffs {
    let w0 = 2.0 * std::f32::consts::PI * frequency / sample_rate;
    let cos_w0 = w0.cos();
    let sin_w0 = w0.sin();
    let q = 0.707;
    let alpha = sin_w0 / (2.0 * q);

    let b0 = (1.0 - cos_w0) / 2.0;
    let b1 = 1.0 - cos_w0;
    let b2 = (1.0 - cos_w0) / 2.0;
    let a0 = 1.0 + alpha;
    let a1 = -2.0 * cos_w0;
    let a2 = 1.0 - alpha;

    normalize_coeffs(b0, b1, b2, a0, a1, a2)
}

fn highpass_coeffs(sample_rate: f32, frequency: f32) -> BiquadCoeffs {
    let w0 = 2.0 * std::f32::consts::PI * frequency / sample_rate;
    let cos_w0 = w0.cos();
    let sin_w0 = w0.sin();
    let q = 0.707;
    let alpha = sin_w0 / (2.0 * q);

    let b0 = (1.0 + cos_w0) / 2.0;
    let b1 = -(1.0 + cos_w0);
    let b2 = (1.0 + cos_w0) / 2.0;
    let a0 = 1.0 + alpha;
    let a1 = -2.0 * cos_w0;
    let a2 = 1.0 - alpha;

    normalize_coeffs(b0, b1, b2, a0, a1, a2)
}

fn normalize_coeffs(b0: f32, b1: f32, b2: f32, a0: f32, a1: f32, a2: f32) -> BiquadCoeffs {
    BiquadCoeffs {
        b0: b0 / a0,
        b1: b1 / a0,
        b2: b2 / a0,
        a1: a1 / a0,
        a2: a2 / a0,
    }
}

/// One-knob DJ filter mapped from the mixer FLT knob range (±24 dB).
///
/// The knob writes [`Self::target_db`]; the audio thread glides
/// [`Self::current_db`] toward it and only swaps coefficients in place, so a
/// sweep never steps the cutoff or wipes the delay line (both click).
#[derive(Debug)]
pub struct DjFilter {
    sample_rate: f32,
    /// Commanded knob position: `-24` full lowpass, `0` bypass, `+24` full highpass.
    target_db: f32,
    /// Glided knob position driving the coefficients and the wet blend.
    current_db: f32,
    /// Position the loaded coefficients belong to; `NaN` forces the first write.
    coeffs_db: f32,
    /// Largest knob travel per sample, sized by [`SWEEP_SECS`].
    step_db: f32,
    lowpass: Biquad,
    highpass: Biquad,
}

impl DjFilter {
    pub fn new(sample_rate: u32) -> Self {
        let sample_rate = sample_rate as f32;
        let mut filter = Self {
            sample_rate,
            target_db: 0.0,
            current_db: 0.0,
            coeffs_db: f32::NAN,
            step_db: (2.0 * EQ_MAX_DB) / (SWEEP_SECS * sample_rate),
            lowpass: Biquad::new(identity_coeffs()),
            highpass: Biquad::new(identity_coeffs()),
        };
        filter.apply_coefficients();
        filter
    }

    /// Commanded knob position in dB (what the UI and soft takeover read back).
    pub fn filter_db(&self) -> f32 {
        self.target_db
    }

    pub fn set_filter_db(&mut self, filter_db: f32) {
        if filter_db.is_finite() {
            self.target_db = clamp_gain_db(filter_db);
        }
    }

    pub fn process_buffer(&mut self, buffer: &mut [Sample]) {
        if self.current_db == 0.0 && self.target_db == 0.0 {
            // Centred: bit-exact dry. Cold state keeps the next engagement smooth.
            self.lowpass.reset();
            self.highpass.reset();
            return;
        }

        for chunk in buffer.chunks_mut(COEFF_UPDATE_FRAMES * 2) {
            let frames = chunk.len() / 2;
            if frames == 0 {
                break;
            }

            let from_db = self.current_db;
            let travel = self.step_db * frames as f32;
            let to_db = if (self.target_db - from_db).abs() <= travel {
                self.target_db
            } else if self.target_db > from_db {
                from_db + travel
            } else {
                from_db - travel
            };
            self.current_db = to_db;
            self.apply_coefficients();

            // The blend moves continuously across the centre, so crossing it
            // swaps LP for HP without a step.
            let (mut low_mix, mut high_mix) = blend(from_db);
            let (to_low, to_high) = blend(to_db);
            let low_step = (to_low - low_mix) / frames as f32;
            let high_step = (to_high - high_mix) / frames as f32;

            for frame in chunk.chunks_mut(2) {
                if frame.len() < 2 {
                    break;
                }
                let (left, right) = (frame[0], frame[1]);
                let (low_left, low_right) = self.lowpass.process_stereo(left, right);
                let (high_left, high_right) = self.highpass.process_stereo(left, right);
                frame[0] = left + low_mix * (low_left - left) + high_mix * (high_left - left);
                frame[1] = right + low_mix * (low_right - right) + high_mix * (high_right - right);
                low_mix += low_step;
                high_mix += high_step;
            }
        }
    }

    fn apply_coefficients(&mut self) {
        // A parked knob keeps its coefficients: only a glide pays the trig.
        if self.coeffs_db == self.current_db {
            return;
        }
        let amount = (self.current_db.abs() / EQ_MAX_DB).clamp(0.05, 1.0);
        let low_cutoff = LP_CUTOFF_HZ + (8_000.0 - LP_CUTOFF_HZ) * (1.0 - amount);
        let high_cutoff = HP_CUTOFF_HZ + (8_000.0 - HP_CUTOFF_HZ) * amount;
        self.lowpass
            .set_coeffs(lowpass_coeffs(self.sample_rate, low_cutoff));
        self.highpass
            .set_coeffs(highpass_coeffs(self.sample_rate, high_cutoff));
        self.coeffs_db = self.current_db;
    }
}

/// Dry/wet weights for a knob position: LP below centre, HP above.
fn blend(db: f32) -> (f32, f32) {
    ((-db).max(0.0) / EQ_MAX_DB, db.max(0.0) / EQ_MAX_DB)
}

pub fn db_to_linear(db: f32) -> f32 {
    10_f32.powf(clamp_gain_db(db) / 20.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eq::EQ_MIN_DB;

    #[test]
    fn bypass_is_identity() {
        let mut filter = DjFilter::new(48_000);
        let mut buffer = vec![0.5, -0.5, 0.25, -0.25];
        filter.process_buffer(&mut buffer);
        assert_eq!(buffer, vec![0.5, -0.5, 0.25, -0.25]);
    }

    /// A knob sweep steps the cutoff between audio callbacks; the output must
    /// stay continuous, or every step is an audible click.
    #[test]
    fn stepping_the_knob_does_not_click() {
        const SR: f32 = 48_000.0;
        let mut filter = DjFilter::new(SR as u32);
        let mut prev = 0.0f32;
        let mut worst = 0.0f32;
        for block in 0..400 {
            // A fast MIDI sweep jumps the knob by whole steps between callbacks.
            filter.set_filter_db(if block % 2 == 0 { -12.0 } else { 12.0 });
            let mut buffer = Vec::with_capacity(512 * 2);
            for frame in 0..512 {
                let t = (block * 512 + frame) as f32 / SR;
                let sample = (2.0 * std::f32::consts::PI * 220.0 * t).sin() * 0.5;
                buffer.push(sample);
                buffer.push(sample);
            }
            filter.process_buffer(&mut buffer);
            for sample in buffer.iter().step_by(2) {
                worst = worst.max((sample - prev).abs());
                prev = *sample;
            }
        }
        // The dry tone's own step is ~0.014; a reset or stepped filter state
        // shows up as a jump of a sizeable fraction of the 0.5 amplitude.
        assert!(worst < 0.05, "worst per-sample step {worst}");
    }

    #[test]
    fn knob_glide_is_gradual_and_reaches_the_target() {
        let mut filter = DjFilter::new(48_000);
        filter.set_filter_db(EQ_MIN_DB);
        // One short block moves a fraction of the way, never the whole sweep.
        let mut short = vec![0.0; 2 * 64];
        filter.process_buffer(&mut short);
        assert!(
            filter.current_db > EQ_MIN_DB / 2.0,
            "jumped straight to {}",
            filter.current_db
        );
        // A full sweep takes SWEEP_SECS; after that the knob is where it was put.
        let mut rest = vec![0.0; 2 * 4_800];
        filter.process_buffer(&mut rest);
        assert_eq!(filter.current_db, EQ_MIN_DB);
        // A non-finite command is ignored instead of poisoning the glide.
        filter.set_filter_db(f32::NAN);
        assert_eq!(filter.filter_db(), EQ_MIN_DB);
    }

    #[test]
    fn lowpass_reduces_high_frequency_energy() {
        let mut filter = DjFilter::new(48_000);
        filter.set_filter_db(EQ_MIN_DB);
        // The cutoff glides in; measure the steady state, not the sweep.
        let mut settle = vec![0.0; 2 * 4_800];
        filter.process_buffer(&mut settle);

        let mut tone = Vec::new();
        for i in 0..512 {
            let t = i as f32 / 48_000.0;
            let sample = (2.0 * std::f32::consts::PI * 4_000.0 * t).sin() * 0.25;
            tone.push(sample);
            tone.push(sample);
        }

        let dry_energy: f32 = tone.iter().map(|s| s * s).sum();
        filter.process_buffer(&mut tone);
        let wet_energy: f32 = tone.iter().map(|s| s * s).sum();
        assert!(wet_energy < dry_energy);
    }
}
