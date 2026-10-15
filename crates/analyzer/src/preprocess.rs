//! Peak normalization of decoded PCM before it reaches a backend.
//!
//! The analyzer-core policy (`docs/audio-analyzer-spec.md` §7) is: peak
//! normalize when `|sample| > 1.0`. Decoders hand back f32 directly from the
//! container, and some masters carry inter-sample peaks above full scale, so the
//! backends need a buffer that is guaranteed to fit `±1.0`.

use analyzer_core::AnalysisConfig;
use std::borrow::Cow;

use crate::decode::DecodedMono;

/// Backends require PCM that fits the ±1.0 encode range; see
/// `docs/audio-analyzer-spec.md` §7. Up to and including this value the buffer
/// is left untouched, which is the common case.
const PEAK_NORMALIZE_THRESHOLD: f32 = 1.0;

/// PCM that is ready to hand to a backend.
pub struct PreparedPcm<'a> {
    /// Mono samples, scaled down when the peak exceeded `±1.0`.
    pub samples: Cow<'a, [f32]>,
    /// Sample rate the samples were decoded at.
    pub sample_rate: u32,
}

/// Prepare PCM that the caller owns.
///
/// The decoder hands this crate a freshly allocated mono buffer nobody else
/// holds a reference to, so normalization can happen in place: no copy of the
/// whole track, and only buffers that actually exceed `±1.0` pay for a pass.
/// Returns the buffer unchanged when it already fits.
pub fn prepare_owned(decoded: DecodedMono) -> PreparedPcm<'static> {
    let mut samples = decoded.samples;
    peak_normalize(&mut samples);
    PreparedPcm {
        samples: Cow::Owned(samples),
        sample_rate: decoded.sample_rate,
    }
}

/// Prepare PCM the caller only borrowed.
///
/// Applies the same policy as [`prepare_owned`]. The copy is avoided when the
/// peak already fits `±1.0`, which is the common case for PCM a caller has
/// already normalized or decoded in range. When it does not, the buffer is
/// scanned twice — once for the peak, once to collect the scaled copy — rather
/// than three times (copy, scan, scale) as before.
pub fn prepare<'a>(
    samples: &'a [f32],
    sample_rate: u32,
    _config: &AnalysisConfig,
) -> PreparedPcm<'a> {
    let peak = peak_of(samples);
    if peak <= PEAK_NORMALIZE_THRESHOLD {
        return PreparedPcm {
            samples: Cow::Borrowed(samples),
            sample_rate,
        };
    }
    let scale = 1.0 / peak;
    PreparedPcm {
        samples: Cow::Owned(samples.iter().map(|sample| sample * scale).collect()),
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
fn peak_of(samples: &[f32]) -> f32 {
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

/// Scale `samples` in place so its peak is exactly [`PEAK_NORMALIZE_THRESHOLD`].
///
/// No-op when the peak is already within range.
fn peak_normalize(samples: &mut [f32]) -> f32 {
    let peak = peak_of(samples);
    scale_down(samples, peak);
    peak
}

/// Scale `samples` in place so the peak is exactly `1.0`.
///
/// No-op when `peak <= 1.0`. `peak` must be the buffer's actual peak, i.e. the
/// value [`peak_of`] reports for the same contents.
fn scale_down(samples: &mut [f32], peak: f32) {
    if peak > PEAK_NORMALIZE_THRESHOLD {
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

    /// The boundary itself: `peak == PEAK_NORMALIZE_THRESHOLD` is in range, so
    /// the buffer is left alone. A refactor to `>=` would silently rescale every
    /// full-scale master.
    #[test]
    fn scale_down_keeps_the_threshold_boundary_inclusive() {
        let mut at_threshold = vec![0.25, PEAK_NORMALIZE_THRESHOLD, -0.5];
        let expected = at_threshold.clone();
        scale_down(&mut at_threshold, PEAK_NORMALIZE_THRESHOLD);
        assert_eq!(
            at_threshold, expected,
            "a peak exactly at the threshold must not be rescaled"
        );

        let mut just_over = vec![0.25, f32::EPSILON * 4.0, -0.5];
        peak_normalize(&mut just_over);
        assert!(
            just_over
                .iter()
                .all(|sample| sample.abs() <= PEAK_NORMALIZE_THRESHOLD),
            "a peak over the threshold must be rescaled down to it"
        );
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

    /// The fused copy-and-rescale has to land on exactly the same floats as a
    /// plain copy followed by the in-place rescale.
    #[test]
    fn prepare_fused_scale_matches_copy_then_scale() {
        let samples: Vec<f32> = (0..5_000)
            .map(|index| (index as f32 / 997.0).sin() * 2.5 - 1.25)
            .collect();
        let peak = peak_of(&samples);
        let mut expected = samples.clone();
        peak_normalize(&mut expected);

        let prepared = prepare(&samples, 44_100, &AnalysisConfig::default());
        assert_eq!(&*prepared.samples, &expected[..]);
        assert!(peak > PEAK_NORMALIZE_THRESHOLD);
    }
}
