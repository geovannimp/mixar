//! Three-band channel EQ (low shelf, mid peak, high shelf).

use anyhow::Result;
use audio_core::Sample;

pub const EQ_MIN_DB: f32 = -24.0;
pub const EQ_MAX_DB: f32 = 24.0;

const LOW_SHELF_HZ: f32 = 180.0;
const MID_CENTER_HZ: f32 = 900.0;
const MID_Q: f32 = 1.4;
const HIGH_SHELF_HZ: f32 = 6_000.0;

/// Per-band gain in decibels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DeckEqGains {
    pub low_db: f32,
    pub mid_db: f32,
    pub high_db: f32,
}

impl Default for DeckEqGains {
    fn default() -> Self {
        Self {
            low_db: 0.0,
            mid_db: 0.0,
            high_db: 0.0,
        }
    }
}

impl DeckEqGains {
    pub fn clamped(low_db: f32, mid_db: f32, high_db: f32) -> Self {
        Self {
            low_db: clamp_gain_db(low_db),
            mid_db: clamp_gain_db(mid_db),
            high_db: clamp_gain_db(high_db),
        }
    }
}

pub fn clamp_gain_db(value: f32) -> f32 {
    value.clamp(EQ_MIN_DB, EQ_MAX_DB)
}

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

    fn set_coeffs(&mut self, coeffs: BiquadCoeffs) {
        self.coeffs = coeffs;
    }

    fn process_sample(coeffs: BiquadCoeffs, input: f32, state: &mut BiquadState) -> f32 {
        let x = input;
        let y = coeffs.b0 * x + state.z1;
        state.z1 = coeffs.b1 * x - coeffs.a1 * y + state.z2;
        state.z2 = coeffs.b2 * x - coeffs.a2 * y;
        y
    }

    fn process_stereo(&mut self, left: f32, right: f32) -> (f32, f32) {
        let coeffs = self.coeffs;
        let l = Self::process_sample(coeffs, left, &mut self.left);
        let r = Self::process_sample(coeffs, right, &mut self.right);
        (l, r)
    }
}

/// DJ-style three-band EQ applied per deck channel.
#[derive(Debug)]
pub struct ThreeBandEq {
    sample_rate: f32,
    gains: DeckEqGains,
    low: Biquad,
    mid: Biquad,
    high: Biquad,
}

impl ThreeBandEq {
    pub fn new(sample_rate: u32) -> Self {
        let mut eq = Self {
            sample_rate: sample_rate as f32,
            gains: DeckEqGains::default(),
            low: Biquad::new(identity_coeffs()),
            mid: Biquad::new(identity_coeffs()),
            high: Biquad::new(identity_coeffs()),
        };
        eq.update_coefficients();
        eq
    }

    pub fn gains(&self) -> DeckEqGains {
        self.gains
    }

    pub fn set_gains(&mut self, gains: DeckEqGains) -> Result<()> {
        self.gains = DeckEqGains::clamped(gains.low_db, gains.mid_db, gains.high_db);
        self.update_coefficients();
        Ok(())
    }

    pub fn set_low_db(&mut self, gain_db: f32) -> Result<()> {
        self.gains.low_db = clamp_gain_db(gain_db);
        self.low.set_coeffs(low_shelf_coeffs(
            self.sample_rate,
            LOW_SHELF_HZ,
            self.gains.low_db,
        ));
        Ok(())
    }

    pub fn set_mid_db(&mut self, gain_db: f32) -> Result<()> {
        self.gains.mid_db = clamp_gain_db(gain_db);
        self.mid
            .set_coeffs(peaking_coeffs(self.sample_rate, MID_CENTER_HZ, MID_Q, self.gains.mid_db));
        Ok(())
    }

    pub fn set_high_db(&mut self, gain_db: f32) -> Result<()> {
        self.gains.high_db = clamp_gain_db(gain_db);
        self.high.set_coeffs(high_shelf_coeffs(
            self.sample_rate,
            HIGH_SHELF_HZ,
            self.gains.high_db,
        ));
        Ok(())
    }

    pub fn process_buffer(&mut self, buffer: &mut [Sample]) {
        for frame in buffer.chunks_mut(2) {
            if frame.len() < 2 {
                break;
            }
            let (left, right) = self.low.process_stereo(frame[0], frame[1]);
            let (left, right) = self.mid.process_stereo(left, right);
            let (left, right) = self.high.process_stereo(left, right);
            frame[0] = left;
            frame[1] = right;
        }
    }

    fn update_coefficients(&mut self) {
        self.low.set_coeffs(low_shelf_coeffs(
            self.sample_rate,
            LOW_SHELF_HZ,
            self.gains.low_db,
        ));
        self.mid
            .set_coeffs(peaking_coeffs(self.sample_rate, MID_CENTER_HZ, MID_Q, self.gains.mid_db));
        self.high.set_coeffs(high_shelf_coeffs(
            self.sample_rate,
            HIGH_SHELF_HZ,
            self.gains.high_db,
        ));
    }
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

fn low_shelf_coeffs(sample_rate: f32, frequency: f32, gain_db: f32) -> BiquadCoeffs {
    if gain_db.abs() < f32::EPSILON {
        return identity_coeffs();
    }

    let a = 10_f32.powf(gain_db / 40.0);
    let w0 = 2.0 * std::f32::consts::PI * frequency / sample_rate;
    let cos_w0 = w0.cos();
    let sin_w0 = w0.sin();
    let s = 1.0;
    let alpha = sin_w0 / 2.0 * ((a + 1.0 / a) * (1.0 / s - 1.0) + 2.0).sqrt();

    let b0 = a * ((a + 1.0) - (a - 1.0) * cos_w0 + 2.0 * a.sqrt() * alpha);
    let b1 = 2.0 * a * ((a - 1.0) - (a + 1.0) * cos_w0);
    let b2 = a * ((a + 1.0) - (a - 1.0) * cos_w0 - 2.0 * a.sqrt() * alpha);
    let a0 = (a + 1.0) + (a - 1.0) * cos_w0 + 2.0 * a.sqrt() * alpha;
    let a1 = -2.0 * ((a - 1.0) + (a + 1.0) * cos_w0);
    let a2 = (a + 1.0) + (a - 1.0) * cos_w0 - 2.0 * a.sqrt() * alpha;

    normalize_coeffs(b0, b1, b2, a0, a1, a2)
}

fn high_shelf_coeffs(sample_rate: f32, frequency: f32, gain_db: f32) -> BiquadCoeffs {
    if gain_db.abs() < f32::EPSILON {
        return identity_coeffs();
    }

    let a = 10_f32.powf(gain_db / 40.0);
    let w0 = 2.0 * std::f32::consts::PI * frequency / sample_rate;
    let cos_w0 = w0.cos();
    let sin_w0 = w0.sin();
    let s = 1.0;
    let alpha = sin_w0 / 2.0 * ((a + 1.0 / a) * (1.0 / s - 1.0) + 2.0).sqrt();

    let b0 = a * ((a + 1.0) + (a - 1.0) * cos_w0 + 2.0 * a.sqrt() * alpha);
    let b1 = -2.0 * a * ((a - 1.0) + (a + 1.0) * cos_w0);
    let b2 = a * ((a + 1.0) + (a - 1.0) * cos_w0 - 2.0 * a.sqrt() * alpha);
    let a0 = (a + 1.0) - (a - 1.0) * cos_w0 + 2.0 * a.sqrt() * alpha;
    let a1 = 2.0 * ((a - 1.0) - (a + 1.0) * cos_w0);
    let a2 = (a + 1.0) - (a - 1.0) * cos_w0 - 2.0 * a.sqrt() * alpha;

    normalize_coeffs(b0, b1, b2, a0, a1, a2)
}

fn peaking_coeffs(sample_rate: f32, frequency: f32, q: f32, gain_db: f32) -> BiquadCoeffs {
    if gain_db.abs() < f32::EPSILON {
        return identity_coeffs();
    }

    let a = 10_f32.powf(gain_db / 40.0);
    let w0 = 2.0 * std::f32::consts::PI * frequency / sample_rate;
    let cos_w0 = w0.cos();
    let sin_w0 = w0.sin();
    let alpha = sin_w0 / (2.0 * q);

    let b0 = 1.0 + alpha * a;
    let b1 = -2.0 * cos_w0;
    let b2 = 1.0 - alpha * a;
    let a0 = 1.0 + alpha / a;
    let a1 = -2.0 * cos_w0;
    let a2 = 1.0 - alpha / a;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unity_gains_are_identity() {
        let mut eq = ThreeBandEq::new(48_000);
        let mut buffer = vec![0.5, -0.5, 0.25, -0.25];
        eq.process_buffer(&mut buffer);
        assert_eq!(buffer, vec![0.5, -0.5, 0.25, -0.25]);
    }

    #[test]
    fn clamps_out_of_range_gains() {
        let gains = DeckEqGains::clamped(24.0, -24.0, 0.0);
        assert_eq!(gains.low_db, EQ_MAX_DB);
        assert_eq!(gains.mid_db, EQ_MIN_DB);
    }

    #[test]
    fn low_boost_changes_bass_energy() {
        let mut eq = ThreeBandEq::new(48_000);
        eq.set_low_db(12.0).unwrap();

        let mut low_tone = Vec::new();
        for i in 0..512 {
            let t = i as f32 / 48_000.0;
            let sample = (2.0 * std::f32::consts::PI * 80.0 * t).sin() * 0.25;
            low_tone.push(sample);
            low_tone.push(sample);
        }

        let dry_energy: f32 = low_tone.iter().map(|s| s * s).sum();
        eq.process_buffer(&mut low_tone);
        let wet_energy: f32 = low_tone.iter().map(|s| s * s).sum();
        assert!(wet_energy > dry_energy);
    }
}
