//! qm-dsp `ConstantQ` + `Chromagram`.
//!
//! The constant-Q kernel is built once (a Hamming-windowed complex sinusoid per
//! bin, FFT'd and thresholded to a sparse matrix), then applied to the
//! FFT-shifted, Hamming-windowed signal frame. Octaves are folded into chroma
//! bins and unit-max normalised. FFTs come from `rustfft`/`realfft`.

use std::f64::consts::PI;

use crate::fft::{ComplexFft, RealFft};
use crate::math::{next_power_of_two, normalise_unit_max};
use crate::window::hamming;

struct SparseKernel {
    /// FFT-bin indices (`is`).
    is: Vec<usize>,
    /// Constant-Q bin indices (`js`).
    js: Vec<usize>,
    real: Vec<f64>,
    imag: Vec<f64>,
}

pub struct ConstantQ {
    uk: usize,
    fft_len: usize,
    kernel: SparseKernel,
}

impl ConstantQ {
    pub fn new(fs: f64, fmin: f64, fmax: f64, bpo: usize, cq_thresh: f64) -> Self {
        let dq = 1.0 / (2f64.powf(1.0 / bpo as f64) - 1.0);
        let uk = (bpo as f64 * (fmax / fmin).log2()).ceil() as usize;
        let fft_len = next_power_of_two((dq * fs / fmin).ceil() as i32) as usize;
        let kernel = build_kernel(fs, fmin, bpo, dq, uk, fft_len, cq_thresh);
        Self {
            uk,
            fft_len,
            kernel,
        }
    }

    pub fn uk(&self) -> usize {
        self.uk
    }

    pub fn fft_len(&self) -> usize {
        self.fft_len
    }

    /// Apply the sparse kernel to a full-length (mirrored) FFT frame.
    pub fn process(&self, fft_re: &[f64], fft_im: &[f64], cq_re: &mut [f64], cq_im: &mut [f64]) {
        for v in cq_re.iter_mut() {
            *v = 0.0;
        }
        for v in cq_im.iter_mut() {
            *v = 0.0;
        }
        for idx in 0..self.kernel.is.len() {
            let col = self.kernel.is[idx];
            if col == 0 {
                continue;
            }
            let row = self.kernel.js[idx];
            let r1 = self.kernel.real[idx];
            let i1 = self.kernel.imag[idx];
            let j = self.fft_len - col;
            let r2 = fft_re[j];
            let i2 = fft_im[j];
            cq_re[row] += r1 * r2 - i1 * i2;
            cq_im[row] += r1 * i2 + i1 * r2;
        }
    }
}

fn build_kernel(
    fs: f64,
    fmin: f64,
    bpo: usize,
    dq: f64,
    uk: usize,
    fft_len: usize,
    cq_thresh: f64,
) -> SparseKernel {
    let mut is = Vec::new();
    let mut js = Vec::new();
    let mut real = Vec::new();
    let mut imag = Vec::new();

    let mut fft = ComplexFft::new(fft_len);
    let mut kre = vec![0.0f64; fft_len];
    let mut kim = vec![0.0f64; fft_len];
    let mut ore = vec![0.0f64; fft_len];
    let mut oim = vec![0.0f64; fft_len];

    let square_threshold = cq_thresh * cq_thresh;
    let half = fft_len / 2;

    for j in (0..uk).rev() {
        kre.fill(0.0);
        kim.fill(0.0);

        let samples_per_cycle = fs / (fmin * 2f64.powf(j as f64 / bpo as f64));
        let window_len = (dq * samples_per_cycle).ceil() as usize;
        let origin = fft_len / 2 - window_len / 2;

        for i in 0..window_len {
            let angle = 2.0 * PI * i as f64 / samples_per_cycle;
            kre[origin + i] = angle.cos();
            kim[origin + i] = angle.sin();
        }

        let win = hamming(window_len);
        for i in 0..window_len {
            kre[origin + i] = kre[origin + i] * win[i] / window_len as f64;
            kim[origin + i] = kim[origin + i] * win[i] / window_len as f64;
        }

        // Input is expected pre-fftshifted.
        for i in 0..half {
            kre.swap(i, i + half);
            kim.swap(i, i + half);
        }

        fft.forward(&kre, &kim, &mut ore, &mut oim);

        for i in 0..fft_len {
            let mag = ore[i] * ore[i] + oim[i] * oim[i];
            if mag <= square_threshold {
                continue;
            }
            is.push(i);
            js.push(j);
            real.push(ore[i] / fft_len as f64);
            imag.push(-oim[i] / fft_len as f64);
        }
    }

    SparseKernel { is, js, real, imag }
}

pub struct ChromagramConfig {
    pub fs: f64,
    pub min: f64,
    pub max: f64,
    pub bpo: usize,
    pub cq_thresh: f64,
}

/// Constant-Q chromagram, unit-max normalised per frame.
pub struct Chromagram {
    bpo: usize,
    uk: usize,
    frame_size: usize,
    cq: ConstantQ,
    window: Vec<f64>,
    fft: RealFft,
    windowbuf: Vec<f64>,
    fft_re: Vec<f64>,
    fft_im: Vec<f64>,
    cq_re: Vec<f64>,
    cq_im: Vec<f64>,
    chroma: Vec<f64>,
}

impl Chromagram {
    pub fn new(cfg: ChromagramConfig) -> Self {
        let octaves = (cfg.max / cfg.min).log2();
        let fmax = cfg.min * 2f64.powf(octaves.ceil());
        let cq = ConstantQ::new(cfg.fs, cfg.min, fmax, cfg.bpo, cfg.cq_thresh);
        let frame_size = cq.fft_len();
        let uk = cq.uk();
        Self {
            bpo: cfg.bpo,
            uk,
            frame_size,
            cq,
            window: hamming(frame_size),
            fft: RealFft::new(frame_size),
            windowbuf: vec![0.0; frame_size],
            fft_re: vec![0.0; frame_size],
            fft_im: vec![0.0; frame_size],
            cq_re: vec![0.0; uk],
            cq_im: vec![0.0; uk],
            chroma: vec![0.0; cfg.bpo],
        }
    }

    pub fn frame_size(&self) -> usize {
        self.frame_size
    }

    /// Process one frame of `frame_size` time-domain samples; returns the
    /// `bpo`-length chroma vector.
    pub fn process(&mut self, data: &[f64]) -> &[f64] {
        for i in 0..self.frame_size {
            self.windowbuf[i] = data[i] * self.window[i];
        }
        let half = self.frame_size / 2;
        for i in 0..half {
            self.windowbuf.swap(i, i + half);
        }

        self.fft
            .forward(&mut self.windowbuf, &mut self.fft_re, &mut self.fft_im);

        // Rebuild the conjugate-symmetric full spectrum that ConstantQ reads.
        for i in 0..half.saturating_sub(1) {
            self.fft_re[self.frame_size - 1 - i] = self.fft_re[i + 1];
            self.fft_im[self.frame_size - 1 - i] = -self.fft_im[i + 1];
        }

        self.cq
            .process(&self.fft_re, &self.fft_im, &mut self.cq_re, &mut self.cq_im);

        for v in self.chroma.iter_mut() {
            *v = 0.0;
        }
        // Fold every CQ bin into its pitch class. Iterating all bins (rather than
        // whole octaves) keeps a partial trailing octave instead of dropping it,
        // though `GetKeyMode` sizes the range so `uk % bpo == 0`.
        for bin in 0..self.uk {
            let r = self.cq_re[bin];
            let im = self.cq_im[bin];
            self.chroma[bin % self.bpo] += (r * r + im * im).sqrt();
        }
        normalise_unit_max(&mut self.chroma);
        &self.chroma
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chromagram_is_unit_max_normalised() {
        let cfg = ChromagramConfig {
            fs: 5512.5,
            min: 128.4,
            max: 2053.0,
            bpo: 36,
            cq_thresh: 0.0054,
        };
        let mut chroma = Chromagram::new(cfg);
        let frame_size = chroma.frame_size();
        // A 440 Hz tone in the frame.
        let tone: Vec<f64> = (0..frame_size)
            .map(|i| (2.0 * PI * 440.0 * i as f64 / 5512.5).sin())
            .collect();
        let values = chroma.process(&tone).to_vec();
        assert_eq!(values.len(), 36);
        let max = values.iter().fold(0.0f64, |a, b| a.max(*b));
        assert!((max - 1.0).abs() < 1e-9, "max = {max}");
    }
}
