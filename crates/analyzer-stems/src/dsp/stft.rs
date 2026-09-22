use anyhow::{bail, Result};
use num_complex::Complex32;
use realfft::RealFftPlanner;

pub const N_FFT: usize = 4096;
pub const HOP_LENGTH: usize = 1024;

/// Real FFT STFT matching Demucs `spec.spectro` / `spec.ispectro` (Hann, centered, normalized).
pub struct Stft {
    n_fft: usize,
    hop_length: usize,
    window: Vec<f32>,
    forward: std::sync::Arc<dyn realfft::RealToComplex<f32>>,
    inverse: std::sync::Arc<dyn realfft::ComplexToReal<f32>>,
    scratch_forward: Vec<Complex32>,
    scratch_inverse: Vec<Complex32>,
}

impl Stft {
    pub fn new(n_fft: usize, hop_length: usize) -> Self {
        let mut planner = RealFftPlanner::<f32>::new();
        let forward = planner.plan_fft_forward(n_fft);
        let inverse = planner.plan_fft_inverse(n_fft);
        let scratch_forward = forward.make_scratch_vec();
        let scratch_inverse = inverse.make_scratch_vec();
        let window = hann_window(n_fft);
        Self {
            n_fft,
            hop_length,
            window,
            forward,
            inverse,
            scratch_forward,
            scratch_inverse,
        }
    }

    /// Forward STFT; output layout `[F, T]` row-major (`index = f * num_frames + t`).
    pub fn forward(&mut self, wave: &[f32]) -> Result<Vec<Complex32>> {
        let padded = reflect_pad(wave, self.n_fft / 2, self.n_fft / 2);
        let num_frames = 1 + padded.len().saturating_sub(self.n_fft) / self.hop_length;
        let bins = self.n_fft / 2 + 1;
        let mut spec = vec![Complex32::default(); bins * num_frames];
        let mut frame = vec![0.0f32; self.n_fft];
        let mut spectrum = self.forward.make_output_vec();

        for t in 0..num_frames {
            let start = t * self.hop_length;
            for (i, sample) in frame.iter_mut().enumerate().take(self.n_fft) {
                *sample = padded.get(start + i).copied().unwrap_or(0.0) * self.window[i];
            }
            self.forward
                .process_with_scratch(&mut frame, &mut spectrum, &mut self.scratch_forward)
                .map_err(|e| anyhow::anyhow!("stft forward fft failed: {e}"))?;
            for f in 0..bins {
                spec[f * num_frames + t] = spectrum[f];
            }
        }
        Ok(spec)
    }

    /// Inverse STFT; `spec` layout matches [`Self::forward`].
    pub fn inverse(&mut self, spec: &[Complex32], out_len: usize) -> Result<Vec<f32>> {
        if spec.is_empty() {
            return Ok(vec![0.0; out_len]);
        }
        let bins = self.n_fft / 2 + 1;
        if !spec.len().is_multiple_of(bins) {
            bail!("spec length {} not divisible by bins {bins}", spec.len());
        }
        let num_frames = spec.len() / bins;
        let padded_len = self.n_fft + self.hop_length * (num_frames - 1);
        let mut acc = vec![0.0f32; padded_len];
        let mut win_acc = vec![0.0f32; padded_len];
        let mut frame = vec![0.0f32; self.n_fft];
        let mut spectrum = self.inverse.make_input_vec();

        for t in 0..num_frames {
            for f in 0..bins {
                spectrum[f] = spec[f * num_frames + t];
            }
            self.inverse
                .process_with_scratch(&mut spectrum, &mut frame, &mut self.scratch_inverse)
                .map_err(|e| anyhow::anyhow!("stft inverse fft failed: {e}"))?;
            let ifft_scale = 1.0 / self.n_fft as f32;
            let start = t * self.hop_length;
            for i in 0..self.n_fft {
                let w = self.window[i];
                let v = frame[i] * ifft_scale * w;
                acc[start + i] += v;
                win_acc[start + i] += w * w;
            }
        }

        let pad = self.n_fft / 2;
        let mut out = vec![0.0f32; out_len];
        for (i, sample) in out.iter_mut().enumerate().take(out_len) {
            let src = i + pad;
            if src < acc.len() && win_acc[src] > 1e-8 {
                *sample = acc[src] / win_acc[src];
            }
        }
        Ok(out)
    }
}

fn hann_window(n_fft: usize) -> Vec<f32> {
    let raw: Vec<f32> = (0..n_fft)
        .map(|i| 0.5 * (1.0 - (2.0 * std::f32::consts::PI * i as f32 / n_fft as f32).cos()))
        .collect();
    let norm: f32 = raw.iter().map(|w| w * w).sum::<f32>().sqrt();
    raw.into_iter().map(|w| w / norm).collect()
}

/// `torch.nn.functional.pad(..., mode="reflect")` on a 1-D signal.
pub(crate) fn reflect_pad(wave: &[f32], left: usize, right: usize) -> Vec<f32> {
    if wave.is_empty() {
        return vec![0.0; left + right];
    }
    let mut out = Vec::with_capacity(wave.len() + left + right);
    for i in (0..left).rev() {
        let idx = (i + 1).min(wave.len() - 1);
        out.push(wave[idx]);
    }
    out.extend_from_slice(wave);
    for i in 0..right {
        let idx = wave.len().saturating_sub(2).saturating_sub(i);
        out.push(wave[idx]);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stft_istft_recovers_sine_roughly() {
        let sr = 44_100;
        let n = 8192;
        let wave: Vec<f32> = (0..n)
            .map(|i| (2.0 * std::f32::consts::PI * 440.0 * i as f32 / sr as f32).sin())
            .collect();
        let mut stft = Stft::new(N_FFT, HOP_LENGTH);
        let spec = stft.forward(&wave).unwrap();
        let back = stft.inverse(&spec, n).unwrap();
        let err: f32 = wave
            .iter()
            .zip(back.iter())
            .map(|(a, b)| (a - b).abs())
            .sum::<f32>()
            / n as f32;
        assert!(err < 0.05, "mean abs err {err}");
    }
}
