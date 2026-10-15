//! qm-dsp `DetectionFunction` (complex-domain onset detection) and the phase
//! vocoder front end it relies on.
//!
//! Mixxx configures this as `DF_COMPLEXSD` with a Hann window. The other
//! detection functions are not ported because Mixxx never selects them; the
//! complex-domain measure is reproduced exactly, including the persistent
//! magnitude/phase history.
//!
//! The qm-dsp measure derives a wrapped *phase* deviation (`atan2`, `princarg`,
//! `cos`/`sin`) per bin. This port instead rotates the previous frame's unit
//! phasors: `magnitude * (cos dev, sin dev)` is the direction of the complex
//! vector `(re + i*im) * conj(prev)^2 * prev_prev`, whose magnitude is
//! `magnitude`. The phasor rotation is algebraically equivalent to the angle
//! formulation (`atan2` + `princarg` + `cos`/`sin`) and agrees with it to
//! within floating-point rounding.

use crate::fft::RealFft;
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
    mag_history: Vec<f64>,
    /// Unit phasor of the previous frame, per bin (`1 + 0i` initially).
    prev_re: Vec<f64>,
    prev_im: Vec<f64>,
    /// Unit phasor of the frame before that, per bin (`1 + 0i`, matching
    /// `phase_history_old` starting at angle 0).
    prev_prev_re: Vec<f64>,
    prev_prev_im: Vec<f64>,
}

impl DetectionFunction {
    /// `frame_length` must be even (Mixxx uses a power of two).
    pub fn new(frame_length: usize) -> Self {
        debug_assert!(frame_length.is_multiple_of(2), "frame_length must be even");
        let half = frame_length / 2 + 1;
        Self {
            n: frame_length,
            half,
            window: hann(frame_length),
            time: vec![0.0; frame_length],
            fft: RealFft::new(frame_length),
            re: vec![0.0; half],
            im: vec![0.0; half],
            mag_history: vec![0.0; half],
            prev_re: vec![1.0; half],
            prev_im: vec![0.0; half],
            // `phase_history_old` starts at angle 0, i.e. `e^{i*0}` = (1, 0).
            prev_prev_re: vec![1.0; half],
            prev_prev_im: vec![0.0; half],
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

        let mut val = 0.0;
        for i in 0..self.half {
            let r = self.re[i];
            let im = self.im[i];
            let mag = (r * r + im * im).sqrt();
            // `magnitude * (cos dev, sin dev)` is the direction of the complex vector
            //   (re + i*im) * conj(prev)^2 * prev_prev
            // (its magnitude is `mag`): qm-dsp's wrapped phase deviation, with no
            // transcendental call.
            let a = self.prev_re[i];
            let b = self.prev_im[i];
            let c0 = a * a - b * b;
            let c1 = -2.0 * a * b;
            let t0 = r * c0 - im * c1;
            let t1 = r * c1 + im * c0;
            let d0 = t0 * self.prev_prev_re[i] - t1 * self.prev_prev_im[i];
            let d1 = t1 * self.prev_prev_re[i] + t0 * self.prev_prev_im[i];
            let tmp_real = self.mag_history[i] - d0;
            let tmp_imag = -d1;
            val += (tmp_real * tmp_real + tmp_imag * tmp_imag).sqrt();

            self.prev_prev_re[i] = a;
            self.prev_prev_im[i] = b;
            self.prev_re[i] = if mag == 0.0 { 1.0 } else { r / mag };
            self.prev_im[i] = if mag == 0.0 { 0.0 } else { im / mag };
            self.mag_history[i] = mag;
        }
        val
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::window::hann;

    #[test]
    fn complex_sd_matches_the_reference_definition() {
        // qm-dsp derives the phase deviation from angles (atan2 + princarg + cos/sin);
        // `DetectionFunction` uses the algebraically equivalent unit-phasor rotation.
        fn princ_arg(ang: f64) -> f64 {
            // qm-dsp `MathUtilities::princarg` — same expression `math::princ_arg` had.
            let y = -2.0 * std::f64::consts::PI;
            let x = ang + std::f64::consts::PI;
            x - y * (x / y).floor() + std::f64::consts::PI
        }
        let n = 512;
        let half = n / 2 + 1;
        let w = crate::window::hann(n);
        let mut df = DetectionFunction::new(n);
        let mut re = vec![0.0; half];
        let mut im = vec![0.0; half];
        let mut mag_hist = vec![0.0; half];
        let mut theta_hist = vec![0.0; half];
        let mut theta_hist_old = vec![0.0; half];
        for f in 0..8u32 {
            let frame: Vec<f64> = (0..n)
                .map(|i| {
                    let t = i as f64;
                    let nf = n as f64;
                    (2.0 * std::f64::consts::PI * (37.0 * f64::from(f) + 5.0) * t / nf).sin() * w[i]
                        + if f % 3 == 0 {
                            0.0
                        } else {
                            (f64::from(f % 7) * (((i * f as usize + 1) % 13) as f64 / 13.0)) - 1.0
                        }
                })
                .collect();
            let got = df.process_time_domain(&frame);

            let mut fft = crate::fft::RealFft::new(n);
            // `process_time_domain` applies its own Hann after `frame` is
            // already windowed, so the reference needs the second multiply too.
            let mut time: Vec<f64> = frame.iter().enumerate().map(|(i, &s)| s * w[i]).collect();
            let hs = n / 2;
            for i in 0..hs {
                time.swap(i, i + hs);
            }
            fft.forward(&mut time, &mut re, &mut im);
            let mut expected = 0.0;
            for i in 0..half {
                let (r, im_i) = (re[i], im[i]);
                let magnitude = (r * r + im_i * im_i).sqrt();
                let theta = im_i.atan2(r);
                let tmp_phase = theta - 2.0 * theta_hist[i] + theta_hist_old[i];
                let dev = princ_arg(tmp_phase);
                let tmp_real = mag_hist[i] - magnitude * dev.cos();
                let tmp_imag = -magnitude * dev.sin();
                expected += (tmp_real * tmp_real + tmp_imag * tmp_imag).sqrt();
                theta_hist_old[i] = theta_hist[i];
                theta_hist[i] = theta;
                mag_hist[i] = magnitude;
            }
            let tol = 1e-9 * expected.abs().max(1.0);
            assert!(
                (got - expected).abs() <= tol,
                "frame {f}: got {got}, reference {expected}"
            );
        }
    }

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
