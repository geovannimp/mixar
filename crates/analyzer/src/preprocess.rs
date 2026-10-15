//! Peak normalization of decoded PCM before it reaches a backend.
//!
//! The analyzer-core policy (`docs/audio-analyzer-spec.md` §7) is: peak
//! normalize when `|sample| > 1.0`. Decoders hand back f32 directly from the
//! container, and some masters carry inter-sample peaks above full scale, so the
//! backends need a buffer that is guaranteed to fit `±1.0`.

use analyzer_core::AnalysisConfig;
use std::borrow::Cow;

use crate::decode::DecodedMono;

/// PCM that is ready to hand to a backend.
pub struct PreparedPcm<'a> {
    /// Mono samples, scaled down when the peak exceeded `±1.0`.
    pub samples: Cow<'a, [f32]>,
    /// Sample rate the samples were decoded at.
    pub sample_rate: u32,
}

/// Prepare PCM that the caller owns, using the peak the decoder already measured.
///
/// The decode stage averages the channels as it goes, so it can track the peak
/// of the mono buffer in the same pass. Handing that peak in here avoids a
/// second full read of the buffer just to recompute it.
///
/// Returns the buffer unchanged when it already fits `±1.0`, so only buffers
/// that actually exceed the range pay for a rescale.
pub fn prepare_owned(decoded: DecodedMono) -> PreparedPcm<'static> {
    let mut samples = decoded.samples;
    scale_down(&mut samples, decoded.peak);
    PreparedPcm {
        samples: Cow::Owned(samples),
        sample_rate: decoded.sample_rate,
    }
}

/// Prepare PCM the caller only borrowed.
///
/// Copies, then applies the same policy as [`prepare_owned`]. The copy is
/// avoided when the peak already fits `±1.0`, which is the common case for PCM
/// a caller has already normalized or decoded in range.
pub fn prepare<'a>(
    samples: &'a [f32],
    sample_rate: u32,
    _config: &AnalysisConfig,
) -> PreparedPcm<'a> {
    let peak = peak_of(samples);
    if peak <= 1.0 {
        return PreparedPcm {
            samples: Cow::Borrowed(samples),
            sample_rate,
        };
    }
    let mut owned = samples.to_vec();
    scale_down(&mut owned, peak);
    PreparedPcm {
        samples: Cow::Owned(owned),
        sample_rate,
    }
}

/// Largest absolute sample in `samples`, or `0.0` when it is empty.
///
/// The reduction is lane-wise over eight samples at a time and then folded
/// horizontally, so the compiler emits a SIMD max per chunk instead of a serial
/// dependency chain through the whole buffer. `f32::max` is commutative and
/// associative, and `0.0` is the identity for the non-negative values being
/// combined, so the result is bit-identical to a plain left-to-right fold —
/// including for NaN inputs, which `max` skips over.
pub fn peak_of(samples: &[f32]) -> f32 {
    /// Samples folded per lane update: one AVX2 `f32x8` register.
    const LANES: usize = 8;

    let mut lanes = [0.0f32; LANES];
    for window in samples.as_chunks::<LANES>().0 {
        for (lane, sample) in lanes.iter_mut().zip(window.iter()) {
            *lane = lane.max(sample.abs());
        }
    }
    for sample in samples.as_chunks::<LANES>().1 {
        lanes[0] = lanes[0].max(sample.abs());
    }

    let mut peak = 0.0f32;
    for &lane in &lanes {
        peak = peak.max(lane);
    }
    peak
}

/// Scale `samples` in place so the peak is exactly `1.0`.
///
/// No-op when `peak <= 1.0`. `peak` must be the buffer's actual peak, i.e. the
/// value [`peak_of`] reports for the same contents.
fn scale_down(samples: &mut [f32], peak: f32) {
    if peak > 1.0 {
        let scale = 1.0 / peak;
        for s in samples {
            *s *= scale;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn peak_of_ignores_sign_and_handles_empty() {
        assert_eq!(peak_of(&[]), 0.0);
        assert_eq!(peak_of(&[0.5, -0.25, 0.125]), 0.5);
        assert_eq!(peak_of(&[0.0, 0.0, -2.0]), 2.0);
    }

    #[test]
    fn peak_of_equals_a_left_to_right_fold() {
        // The lane-wise SIMD reduction must agree with the obvious scalar fold,
        // bit for bit, including where NaN and signed zeros appear.
        let mut rng = 0x9E3779B97F4A7C15u64;
        let mut next = || {
            rng ^= rng << 13;
            rng ^= rng >> 7;
            rng ^= rng << 17;
            rng as f32 / u64::MAX as f32
        };
        let mut samples: Vec<f32> = (0..5_000).map(|_| next() * 4.0 - 2.0).collect();
        samples.extend_from_slice(&[0.0, -0.0, f32::NAN, 1.5, -1.5, 0.0]);
        samples.extend_from_slice(&[(0..200).map(|_| next()).sum::<f32>()]);

        let reference = samples
            .iter()
            .fold(0.0f32, |peak, sample| peak.max(sample.abs()));
        assert_eq!(peak_of(&samples), reference);
    }

    #[test]
    fn scale_down_reaches_exactly_one() {
        let mut samples = vec![0.0, 2.0, -2.0];
        scale_down(&mut samples, 2.0);
        assert_eq!(samples, vec![0.0, 1.0, -1.0]);
    }

    #[test]
    fn scale_down_negative_only_peak() {
        let mut samples = vec![0.25, 0.0, 0.0, -4.0];
        scale_down(&mut samples, 4.0);
        assert_eq!(samples, vec![0.0625, 0.0, 0.0, -1.0]);
    }

    #[test]
    fn scale_down_is_a_noop_in_range() {
        let mut samples = vec![0.0, 1.0, -1.0, 0.5, -0.75];
        let unchanged = samples.clone();
        scale_down(&mut samples, 1.0);
        assert_eq!(samples, unchanged);

        let mut zeros = vec![0.0; 4];
        scale_down(&mut zeros, 0.0);
        assert_eq!(zeros, vec![0.0; 4]);
    }

    #[test]
    fn prepare_borrows_when_already_in_range() {
        let samples = vec![0.5f32, -0.25, 0.75];
        let prepared = prepare(&samples, 44_100, &AnalysisConfig::default());
        assert!(
            matches!(prepared.samples, Cow::Borrowed(_)),
            "in-range PCM must not be copied"
        );
        assert_eq!(&*prepared.samples, &samples[..]);
        assert_eq!(prepared.sample_rate, 44_100);
    }

    #[test]
    fn prepare_copies_only_to_rescale() {
        let samples = vec![0.0f32, 4.0, -2.0];
        let prepared = prepare(&samples, 48_000, &AnalysisConfig::default());
        assert!(matches!(prepared.samples, Cow::Owned(_)));
        assert_eq!(&*prepared.samples, &[0.0f32, 1.0, -0.5]);
        // The caller's buffer is untouched.
        assert_eq!(samples, vec![0.0f32, 4.0, -2.0]);
    }
}
