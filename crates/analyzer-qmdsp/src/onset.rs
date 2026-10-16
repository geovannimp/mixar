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

/// Phasor for a bin the FFT reported as exactly zero.
///
/// Only a *negative* zero real part can point anywhere but angle 0, so the
/// common case (`realfft` emits `+0.0 + 0.0i` for a silent bin, giving
/// `atan2(0.0, 0.0) = 0.0`) is the constant `(1, 0)` and costs nothing. The
/// `atan2` path is kept, but only when the sign of the real part says the
/// answer really is ±pi — a bit test, not a transcendental.
///
/// A silent corpus measured 150.3-152.3 ms on `sub_onset` before this guard and
/// this row is otherwise the crate's dominant stage, so paying `atan2` + `cos` +
/// `sin` per bin on every silent frame was a 60% regression on silence-heavy
/// input.
#[inline]
fn zero_magnitude_phasor(re: f64, im: f64) -> (f64, f64) {
    if re.is_sign_negative() {
        // atan2(+0.0, -0.0) = pi and atan2(-0.0, -0.0) = -pi.
        let theta = im.atan2(re);
        (theta.cos(), theta.sin())
    } else {
        (1.0, 0.0)
    }
}

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
    /// Unit phasor of the frame before that, per bin (`1 + 0i` initially —
    /// qm-dsp's `phase_history_old` started at angle 0).
    prev_prev_re: Vec<f64>,
    prev_prev_im: Vec<f64>,
}

impl DetectionFunction {
    /// Build a detector for `frame_length` samples.
    ///
    /// `frame_length` must be a non-zero power of two (Mixxx uses 1024 at
    /// 44.1 kHz). Odd or zero lengths are not supported: the FFT planners and
    /// the `n/2 + 1` bin count assume it, and the correct answer is undefined
    /// rather than merely slow. Every caller in this crate derives the length
    /// from `measurement::frame_and_step`, which is always a power of two, so
    /// the check stays `debug_assert!` — a release `assert!` would abort the
    /// host process over an input the library itself cannot produce.
    pub fn new(frame_length: usize) -> Self {
        debug_assert!(
            frame_length.is_power_of_two(),
            "frame_length must be a non-zero power of two"
        );
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
            // qm-dsp's `phase_history_old` started at angle 0, i.e. `e^{i*0}` = (1, 0).
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
            // A zero-magnitude bin has no direction to normalise. Only a
            // negative-zero real part can point anywhere but angle 0, so the
            // guard is a bit test rather than a transcendental on the common
            // silent-bin path.
            let (prev_re, prev_im) = if mag == 0.0 {
                zero_magnitude_phasor(r, im)
            } else {
                (r / mag, im / mag)
            };
            self.prev_re[i] = prev_re;
            self.prev_im[i] = prev_im;
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
    fn zero_magnitude_phasor_matches_atan2_for_every_signed_zero() {
        // The four signed-zero combinations are the only inputs this function
        // can see: the caller reaches it only when `mag == 0.0`, which forces
        // both components to be zero (possibly negative zero).
        let zeros = [(0.0f64, 0.0f64), (0.0, -0.0), (-0.0, 0.0), (-0.0, -0.0)];
        for (re, im) in zeros {
            assert_eq!(re * re + im * im, 0.0, "test case is not zero-magnitude");
            let (r, i) = zero_magnitude_phasor(re, im);
            let theta = im.atan2(re);
            let (want_r, want_i) = (theta.cos(), theta.sin());
            // `sin(-0.0) = -0.0` differs from `0.0` in bits only, and compares
            // equal; the phasor arithmetic treats them identically.
            assert_eq!(r, want_r, "re for ({re}, {im})");
            assert!((i - want_i).abs() <= 1e-300, "im for ({re}, {im})");
            // And the phasor is a unit phasor either way.
            assert!((r * r + i * i - 1.0).abs() < 1e-15);
        }
        // The only case that is not the constant `(1, 0)`.
        assert_eq!(zero_magnitude_phasor(-0.0, 0.0).0, -1.0);
        assert_eq!(zero_magnitude_phasor(0.0, 0.0), (1.0, 0.0));
    }

    #[test]
    fn silent_frames_carry_state_into_audio() {
        // The zero-magnitude branch must hand the next frame a phasor the
        // angle formulation would have recorded, so a silent frame followed by
        // audio matches the reference.
        let n = 512;
        let w = hann(n);
        let mut df = DetectionFunction::new(n);
        let half = n / 2 + 1;
        let mut re = vec![0.0; half];
        let mut im = vec![0.0; half];
        let mut mag_hist = vec![0.0; half];
        let mut theta_hist = vec![0.0; half];
        let mut theta_hist_old = vec![0.0; half];

        let frames: Vec<Vec<f64>> = (0..5u32)
            .map(|f| {
                (0..n)
                    .map(|i| {
                        let t = i as f64;
                        let nf = n as f64;
                        let tone = (2.0 * std::f64::consts::PI * (11.0 * f64::from(f) + 3.0) * t
                            / nf)
                            .sin();
                        // Frames 0 and 1 are exactly silent; the rest are audio.
                        if f < 2 {
                            0.0
                        } else {
                            tone * 0.5 + if f == 4 { 0.1 } else { 0.0 }
                        }
                    })
                    .collect()
            })
            .collect();

        for (f, frame) in frames.iter().enumerate() {
            let got = df.process_time_domain(frame);

            let mut fft = crate::fft::RealFft::new(n);
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
                let dev = {
                    let y = -2.0 * std::f64::consts::PI;
                    let x = theta - 2.0 * theta_hist[i] + theta_hist_old[i] + std::f64::consts::PI;
                    x - y * (x / y).floor() + std::f64::consts::PI
                };
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
