//! FFT wrappers.
//!
//! qm-dsp uses kissfft through `FFT` (full complex) and `FFTReal` (real input,
//! mirrored output). We use the established `rustfft`/`realfft` crates instead
//! of porting kissfft; the outputs feed the same downstream arithmetic.

use std::sync::Arc;

use realfft::{RealFftPlanner, RealToComplex};
use rustfft::num_complex::Complex;
use rustfft::{Fft, FftPlanner};

/// Real-input forward FFT, writing bins `0..=n/2`.
pub struct RealFft {
    fft: Arc<dyn RealToComplex<f64>>,
    output: Vec<Complex<f64>>,
    scratch: Vec<Complex<f64>>,
}

impl RealFft {
    pub fn new(n: usize) -> Self {
        let mut planner = RealFftPlanner::<f64>::new();
        let fft = planner.plan_fft_forward(n);
        let output = fft.make_output_vec();
        let scratch = fft.make_scratch_vec();
        Self {
            fft,
            output,
            scratch,
        }
    }

    /// Forward transform of `input` (length `n`); fills `re`/`im` with the
    /// `n/2 + 1` non-redundant bins. `re`/`im` must be at least `n/2 + 1` long.
    pub fn forward(&mut self, input: &mut [f64], re: &mut [f64], im: &mut [f64]) {
        debug_assert_eq!(input.len(), self.fft.len(), "real FFT input length");
        debug_assert!(
            re.len() >= self.output.len() && im.len() >= self.output.len(),
            "real FFT output buffers too small"
        );
        self.fft
            .process_with_scratch(input, &mut self.output, &mut self.scratch)
            .expect("realfft: buffer length mismatch");
        for (i, c) in self.output.iter().enumerate() {
            re[i] = c.re;
            im[i] = c.im;
        }
    }
}

/// Full complex forward FFT (used for the constant-Q kernel).
pub struct ComplexFft {
    fft: Arc<dyn Fft<f64>>,
    buffer: Vec<Complex<f64>>,
    scratch: Vec<Complex<f64>>,
}

impl ComplexFft {
    pub fn new(n: usize) -> Self {
        let mut planner = FftPlanner::<f64>::new();
        let fft = planner.plan_fft_forward(n);
        let scratch = vec![Complex::new(0.0, 0.0); fft.get_inplace_scratch_len()];
        Self {
            fft,
            buffer: vec![Complex::new(0.0, 0.0); n],
            scratch,
        }
    }

    /// Forward transform of a complex signal; writes all `n` bins. `re`, `im`,
    /// `out_re` and `out_im` must each be `n` long.
    pub fn forward(&mut self, re: &[f64], im: &[f64], out_re: &mut [f64], out_im: &mut [f64]) {
        debug_assert_eq!(re.len(), self.buffer.len(), "complex FFT input length");
        debug_assert_eq!(im.len(), self.buffer.len(), "complex FFT input length");
        debug_assert!(
            out_re.len() >= self.buffer.len() && out_im.len() >= self.buffer.len(),
            "complex FFT output buffers too small"
        );
        for (i, slot) in self.buffer.iter_mut().enumerate() {
            *slot = Complex::new(re[i], im[i]);
        }
        self.fft
            .process_with_scratch(&mut self.buffer, &mut self.scratch);
        for (i, c) in self.buffer.iter().enumerate() {
            out_re[i] = c.re;
            out_im[i] = c.im;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::PI;

    #[test]
    fn real_fft_finds_a_sine_bin() {
        let n = 64;
        let bin = 5;
        let mut input: Vec<f64> = (0..n)
            .map(|i| (2.0 * PI * bin as f64 * i as f64 / n as f64).sin())
            .collect();
        let mut fft = RealFft::new(n);
        let mut re = vec![0.0; n / 2 + 1];
        let mut im = vec![0.0; n / 2 + 1];
        fft.forward(&mut input, &mut re, &mut im);
        let mags: Vec<f64> = (0..=n / 2)
            .map(|i| (re[i] * re[i] + im[i] * im[i]).sqrt())
            .collect();
        let peak = mags
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.total_cmp(b.1))
            .unwrap()
            .0;
        assert_eq!(peak, bin);
    }

    #[test]
    fn complex_fft_matches_documented_dc() {
        let n = 8;
        let mut fft = ComplexFft::new(n);
        let re = vec![1.0; n];
        let im = vec![0.0; n];
        let mut out_re = vec![0.0; n];
        let mut out_im = vec![0.0; n];
        fft.forward(&re, &im, &mut out_re, &mut out_im);
        assert!((out_re[0] - n as f64).abs() < 1e-9);
    }
}
