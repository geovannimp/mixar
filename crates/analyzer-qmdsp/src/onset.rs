//! qm-dsp `DetectionFunction` (complex-domain onset detection) and the phase
//! vocoder front end it relies on.
//!
//! Mixxx configures this as `DF_COMPLEXSD` with a Hann window. The other
//! detection functions are not ported because Mixxx never selects them; the
//! complex-domain measure is reproduced exactly, including the persistent
//! magnitude/phase history.

use crate::fft::RealFft;
use crate::math::princ_arg;
use crate::window::hann;

/// Stateful complex-domain detection function.
pub struct DetectionFunction {
    n: usize,
    half: usize,
    window: Vec<f64>,
    /// fftshift scratch (time domain).
    time: Vec<f64>,
    fft: RealFft,
    re: Vec<f64>,
    im: Vec<f64>,
    magnitude: Vec<f64>,
    theta: Vec<f64>,
    mag_history: Vec<f64>,
    phase_history: Vec<f64>,
    phase_history_old: Vec<f64>,
}

impl DetectionFunction {
    /// `frame_length` must be even (Mixxx uses a power of two).
    pub fn new(frame_length: usize) -> Self {
        let half = frame_length / 2 + 1;
        Self {
            n: frame_length,
            half,
            window: hann(frame_length),
            time: vec![0.0; frame_length],
            fft: RealFft::new(frame_length),
            re: vec![0.0; half],
            im: vec![0.0; half],
            magnitude: vec![0.0; half],
            theta: vec![0.0; half],
            mag_history: vec![0.0; half],
            phase_history: vec![0.0; half],
            phase_history_old: vec![0.0; half],
        }
    }

    /// Process one windowed frame (length `frame_length`) and return the
    /// complex-domain detection value.
    pub fn process_time_domain(&mut self, samples: &[f64]) -> f64 {
        for i in 0..self.n {
            self.time[i] = samples[i] * self.window[i];
        }
        let hs = self.n / 2;
        for i in 0..hs {
            self.time.swap(i, i + hs);
        }
        self.fft.forward(&mut self.time, &mut self.re, &mut self.im);
        for i in 0..self.half {
            let r = self.re[i];
            let im = self.im[i];
            self.magnitude[i] = (r * r + im * im).sqrt();
            self.theta[i] = im.atan2(r);
        }
        self.complex_sd()
    }

    /// qm-dsp `DetectionFunction::complexSD`.
    fn complex_sd(&mut self) -> f64 {
        let mut val = 0.0;
        for i in 0..self.half {
            let tmp_phase = self.theta[i] - 2.0 * self.phase_history[i] + self.phase_history_old[i];
            let dev = princ_arg(tmp_phase);
            let tmp_real = self.mag_history[i] - self.magnitude[i] * dev.cos();
            let tmp_imag = -self.magnitude[i] * dev.sin();
            val += (tmp_real * tmp_real + tmp_imag * tmp_imag).sqrt();

            self.phase_history_old[i] = self.phase_history[i];
            self.phase_history[i] = self.theta[i];
            self.mag_history[i] = self.magnitude[i];
        }
        val
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::window::hann;

    #[test]
    fn silence_yields_zero() {
        let mut df = DetectionFunction::new(512);
        let frame = vec![0.0; 512];
        assert_eq!(df.process_time_domain(&frame), 0.0);
    }

    #[test]
    fn onset_raises_the_detection_value() {
        let mut df = DetectionFunction::new(1024);
        let silence = vec![0.0; 1024];
        let _ = df.process_time_domain(&silence);
        // A broadband burst should produce a positive detection value.
        let burst: Vec<f64> = (0..1024)
            .map(|i| if i % 2 == 0 { 0.5 } else { -0.5 })
            .collect();
        let value = df.process_time_domain(&burst);
        assert!(value > 0.0, "value = {value}");
        // Window sanity: DC-free Hann sums to ~n/2.
        let w = hann(1024);
        assert!((w.iter().sum::<f64>() - 512.0).abs() < 1.0);
    }
}
