//! Small numeric helpers ported from qm-dsp's `MathUtilities`.
//!
//! Only the handful of routines the beat/key paths actually use are ported; the
//! arithmetic order matches the C++ so the port tracks Mixxx closely.

use std::f64::consts::PI;

/// `MathUtilities::princarg`: wrap a phase angle into a 2π interval (qm-dsp
/// maps π to π, i.e. the interval is effectively `(-π, π]`).
pub fn princ_arg(ang: f64) -> f64 {
    mod_floor(ang + PI, -2.0 * PI) + PI
}

/// Floating-point modulus with floor semantics (`x - y * floor(x / y)`).
fn mod_floor(x: f64, y: f64) -> f64 {
    x - y * (x / y).floor()
}

/// `MathUtilities::isPowerOfTwo`.
pub fn is_power_of_two(x: i32) -> bool {
    x >= 1 && (x & (x - 1)) == 0
}

/// `MathUtilities::nextPowerOfTwo`.
pub fn next_power_of_two(x: i32) -> i32 {
    if is_power_of_two(x) {
        return x;
    }
    if x < 1 {
        return 1;
    }
    let mut x = x;
    let mut n = 1;
    while x != 0 {
        x >>= 1;
        n <<= 1;
    }
    n
}

/// `MathUtilities::mean`.
pub fn mean(data: &[f64]) -> f64 {
    if data.is_empty() {
        return 0.0;
    }
    let mut sum = 0.0;
    for &v in data {
        sum += v;
    }
    sum / data.len() as f64
}

/// `MathUtilities::mean(data, start, count)` — sequential sum, as in qm-dsp.
pub fn mean_range(data: &[f64], start: usize, count: usize) -> f64 {
    if count == 0 {
        return 0.0;
    }
    let mut sum = 0.0;
    for i in 0..count {
        sum += data[start + i];
    }
    sum / count as f64
}

/// `MathUtilities::getMax` (array form): running max seeded with `data[0]`,
/// updated on strict `>`. Returns `(index, max)`. Empty input yields `(0, 0.0)`.
pub fn get_max_index(data: &[f64]) -> (usize, f64) {
    let mut max = if data.is_empty() { 0.0 } else { data[0] };
    let mut index = 0;
    for (i, &v) in data.iter().enumerate() {
        if v > max {
            max = v;
            index = i;
        }
    }
    (index, max)
}

/// TempoTrackV2's `get_max_val`: max seeded with `0.0` (so all-negative input
/// yields `0.0`), updated on strict `>`.
pub fn max_val(data: &[f64]) -> f64 {
    let mut max = 0.0;
    for &v in data {
        if max < v {
            max = v;
        }
    }
    max
}

/// TempoTrackV2's `get_max_ind`: argmax seeded with `0`, updated on strict `>`.
pub fn max_ind(data: &[f64]) -> usize {
    let mut max = 0.0;
    let mut ind = 0;
    for (i, &v) in data.iter().enumerate() {
        if max < v {
            max = v;
            ind = i;
        }
    }
    ind
}

/// In-place unit-max normalisation (`NormaliseUnitMax`).
pub fn normalise_unit_max(data: &mut [f64]) {
    let mut max = 0.0;
    for &v in data.iter() {
        if v.abs() > max {
            max = v.abs();
        }
    }
    if max != 0.0 {
        for v in data.iter_mut() {
            *v /= max;
        }
    }
}

/// `MathUtilities::adaptiveThreshold` (moving mean with `p_pre = 8`,
/// `p_post = 7`, then rectify).
pub fn adaptive_threshold(data: &mut [f64]) {
    let sz = data.len();
    if sz == 0 {
        return;
    }
    let mut smoothed = vec![0.0f64; sz];
    for i in 0..sz {
        let first = i.saturating_sub(8);
        let last = (i + 7).min(sz - 1);
        smoothed[i] = mean_range(data, first, last - first + 1);
    }
    for i in 0..sz {
        let v = data[i] - smoothed[i];
        data[i] = if v < 0.0 { 0.0 } else { v };
    }
}

/// MIDI pitch → frequency (`Pitch::getFrequencyForPitch`).
pub fn pitch_frequency(midi_pitch: i32, cents_offset: f64, concert_a: f64) -> f64 {
    let p = midi_pitch as f64 + cents_offset / 100.0;
    concert_a * 2f64.powf((p - 69.0) / 12.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn next_power_of_two_matches_qm_dsp() {
        assert_eq!(next_power_of_two(882), 1024);
        assert_eq!(next_power_of_two(2208), 4096);
        assert_eq!(next_power_of_two(1024), 1024);
        assert_eq!(next_power_of_two(0), 1);
    }

    #[test]
    fn princ_arg_wraps_into_range() {
        assert!((princ_arg(0.0) - 0.0).abs() < 1e-12);
        // qm-dsp maps the +pi boundary to +pi (interval is effectively (-pi, pi]).
        assert!((princ_arg(PI) - PI).abs() < 1e-12);
        assert!(princ_arg(3.0 * PI).abs() <= PI + 1e-12);
    }

    #[test]
    fn adaptive_threshold_rectifies() {
        let mut data = vec![0.0, 0.0, 10.0, 0.0, 0.0];
        adaptive_threshold(&mut data);
        assert!(data.iter().all(|&v| v >= 0.0));
        assert!(data[2] > 0.0);
    }

    #[test]
    fn pitch_a4_is_concert_a() {
        assert!((pitch_frequency(69, 0.0, 440.0) - 440.0).abs() < 1e-9);
    }
}
