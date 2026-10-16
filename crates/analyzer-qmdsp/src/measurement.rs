//! The framing and trimming rules the beat pipeline applies.
//!
//! Published (as `analyzer_qmdsp::measurement`, behind the `sub` feature) purely
//! so `examples/qmdsp_timing.rs` can time the same stages the analyzer runs.
//! The example is a separate crate, so it cannot reach a private helper — and
//! hand-inlining the numbers instead would let the harness silently drift from
//! the pipeline it is meant to attribute changes to.
//!
//! `detect_beats` calls every function here, so there is exactly one
//! implementation of each rule.

/// Mixxx's onset step (`kStepSecs`): ~12 ms, 86 Hz resolution.
pub(super) const STEP_SECS: f64 = 0.01161;
/// Mixxx's `kMaximumBinSizeHz` (max analysis window frequency).
pub(super) const MAXIMUM_BIN_SIZE_HZ: u32 = 50;

/// `(window, step)` the onset detection function is framed with, exactly as
/// `detect_beats` derives them.
///
/// Requires `sample_rate > 0`. A zero step size is not meaningful for onset
/// detection, so with `sample_rate == 0` this returns `(1, 0)` rather than
/// panicking: `detect_beats` treats a zero step as "no beats" and returns empty,
/// and `analyze_pcm` rejects a zero sample rate with an error before this is
/// ever reached. The `(1, 0)` shape is deliberate so the helper stays pure; the
/// `framing_step_zero_means_no_beats` test pins it.
pub fn frame_and_step(sample_rate: u32) -> (usize, usize) {
    let step = (sample_rate as f64 * STEP_SECS) as i32;
    let window = crate::math::next_power_of_two((sample_rate / MAXIMUM_BIN_SIZE_HZ) as i32);
    (window.max(0) as usize, step.max(0) as usize)
}

/// The detection-function slice the beat tracker consumes: trailing non-positive
/// frames trimmed, then the first two skipped (Mixxx's noise guard).
pub fn beat_tracking_slice(results: &[f64]) -> &[f64] {
    let mut nonzero = results.len();
    while nonzero > 0 && results[nonzero - 1] <= 0.0 {
        nonzero -= 1;
    }
    if nonzero >= 2 {
        &results[2..nonzero]
    } else {
        &[]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn framing_matches_the_qm_dsp_defaults_at_44k1() {
        // step = 44100 * 0.01161 = 512.0; window = next_pow2(44100/50 = 882) = 1024.
        assert_eq!(frame_and_step(44_100), (1024, 512));
    }

    #[test]
    fn framing_step_zero_means_no_beats() {
        // Not a panic: `detect_beats` treats a zero step as "no beats" and
        // `analyze_pcm` rejects a zero sample rate before this is reached.
        assert_eq!(frame_and_step(0), (1, 0));
    }

    #[test]
    fn beat_tracking_slice_trims_then_skips_two() {
        let results = [1.0, 2.0, 3.0, 4.0, 0.0, 0.0];
        // Both trailing zeros are trimmed, then the first two of the rest dropped.
        assert_eq!(beat_tracking_slice(&results), &[3.0, 4.0]);
    }

    #[test]
    fn beat_tracking_slice_handles_too_few_frames() {
        assert!(beat_tracking_slice(&[1.0]).is_empty());
        assert!(beat_tracking_slice(&[]).is_empty());
        // All non-positive: nothing survives the trim.
        assert!(beat_tracking_slice(&[0.0, 0.0, -1.0]).is_empty());
    }
}
