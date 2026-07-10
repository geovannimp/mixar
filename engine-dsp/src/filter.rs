//! DJ-style one-knob filter (LP left, HP right, center = bypass).

use audio_core::Sample;

use crate::eq::{clamp_gain_db, EQ_MAX_DB};

const LP_CUTOFF_HZ: f32 = 180.0;
const HP_CUTOFF_HZ: f32 = 1_800.0;

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
#[derive(Debug)]
pub struct DjFilter {
    sample_rate: f32,
    filter_db: f32,
    lowpass: Biquad,
    highpass: Biquad,
}

impl DjFilter {
    pub fn new(sample_rate: u32) -> Self {
        let mut filter = Self {
            sample_rate: sample_rate as f32,
            filter_db: 0.0,
            lowpass: Biquad::new(identity_coeffs()),
            highpass: Biquad::new(identity_coeffs()),
        };
        filter.update_coefficients();
        filter
    }

    pub fn filter_db(&self) -> f32 {
        self.filter_db
    }

    pub fn set_filter_db(&mut self, filter_db: f32) {
        self.filter_db = clamp_gain_db(filter_db);
        self.update_coefficients();
    }

    pub fn process_buffer(&mut self, buffer: &mut [Sample]) {
        if self.filter_db.abs() < f32::EPSILON {
            return;
        }

        let wet = (self.filter_db.abs() / EQ_MAX_DB).clamp(0.0, 1.0);
        let use_lowpass = self.filter_db < 0.0;

        for frame in buffer.chunks_mut(2) {
            if frame.len() < 2 {
                break;
            }
            let dry_l = frame[0];
            let dry_r = frame[1];
            let (wet_l, wet_r) = if use_lowpass {
                self.lowpass.process_stereo(dry_l, dry_r)
            } else {
                self.highpass.process_stereo(dry_l, dry_r)
            };
            frame[0] = dry_l * (1.0 - wet) + wet_l * wet;
            frame[1] = dry_r * (1.0 - wet) + wet_r * wet;
        }
    }

    fn update_coefficients(&mut self) {
        if self.filter_db.abs() < f32::EPSILON {
            self.lowpass = Biquad::new(identity_coeffs());
            self.highpass = Biquad::new(identity_coeffs());
            return;
        }

        let amount = (self.filter_db.abs() / EQ_MAX_DB).clamp(0.05, 1.0);
        let lp_cutoff = LP_CUTOFF_HZ + (8_000.0 - LP_CUTOFF_HZ) * (1.0 - amount);
        let hp_cutoff = HP_CUTOFF_HZ + (8_000.0 - HP_CUTOFF_HZ) * amount;
        self.lowpass = Biquad::new(lowpass_coeffs(self.sample_rate, lp_cutoff));
        self.highpass = Biquad::new(highpass_coeffs(self.sample_rate, hp_cutoff));
    }
}

pub fn db_to_linear(db: f32) -> f32 {
    10_f32.powf(clamp_gain_db(db) / 20.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bypass_is_identity() {
        let mut filter = DjFilter::new(48_000);
        let mut buffer = vec![0.5, -0.5, 0.25, -0.25];
        filter.process_buffer(&mut buffer);
        assert_eq!(buffer, vec![0.5, -0.5, 0.25, -0.25]);
    }

    #[test]
    fn lowpass_reduces_high_frequency_energy() {
        let mut filter = DjFilter::new(48_000);
        filter.set_filter_db(EQ_MIN_DB);

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
