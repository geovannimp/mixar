//! Periodic window functions matching qm-dsp's `Window`.
//!
//! qm-dsp's cosine windows are *periodic* (a size-`N` window equals a symmetric
//! size-`N+1` window with the last sample dropped), so the usual symmetric
//! library windows would not be a drop-in. Only Hann (onset STFT) and Hamming
//! (CQT kernel + chroma STFT) are needed.

use std::f64::consts::PI;

/// Periodic Hann window: `0.5 - 0.5 * cos(2*pi*i/n)`.
pub fn hann(n: usize) -> Vec<f64> {
    let mut w = vec![1.0f64; n];
    if n > 1 {
        for (i, v) in w.iter_mut().enumerate() {
            *v = 0.5 - 0.5 * (2.0 * PI * i as f64 / n as f64).cos();
        }
    }
    w
}

/// Periodic Hamming window: `0.54 - 0.46 * cos(2*pi*i/n)`.
pub fn hamming(n: usize) -> Vec<f64> {
    let mut w = vec![1.0f64; n];
    if n > 1 {
        for (i, v) in w.iter_mut().enumerate() {
            *v = 0.54 - 0.46 * (2.0 * PI * i as f64 / n as f64).cos();
        }
    }
    w
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hann_edges_are_zero() {
        let w = hann(8);
        assert!(w[0].abs() < 1e-12);
        assert!((w[4] - 1.0).abs() < 1e-12);
    }

    #[test]
    fn hamming_matches_definition() {
        let w = hamming(16);
        for (i, v) in w.iter().enumerate() {
            let expected = 0.54 - 0.46 * (2.0 * std::f64::consts::PI * i as f64 / 16.0).cos();
            assert!((v - expected).abs() < 1e-12);
        }
    }
}
