//! Integrated EBU R128 loudness for the analysis pipeline.
//!
//! [`UNMEASURABLE_LOUDNESS`] is the message both the measurement and its caller
//! agree on: silence (and anything else below the R128 absolute gate) makes
//! `loudness_global` report `-inf`, which is not a loudness a deck can load
//! with. Keeping the string in one place means the caller's classification of
//! that case cannot drift from the value it is testing.

use analyzer_core::AnalyzerError;
use ebur128::{EbuR128, Mode};

/// [`AnalyzerError::Analysis`] message used when loudness cannot be measured.
pub const UNMEASURABLE_LOUDNESS: &str = "non-finite loudness";

/// Measure integrated EBU R128 loudness for mono PCM.
///
/// # Errors
///
/// Returns [`AnalyzerError::Analysis`] when the meter cannot be built or fed, and
/// the same variant with an "unmeasurable loudness" message when the result is
/// not finite (silence, or a clip whose whole programme sits below the R128
/// absolute gate).
pub fn integrated_lufs_mono(samples: &[f32], sample_rate: u32) -> Result<f64, AnalyzerError> {
    let mut meter = EbuR128::new(1, sample_rate, Mode::I)
        .map_err(|error| AnalyzerError::Analysis(error.to_string()))?;
    meter
        .add_frames_f32(samples)
        .map_err(|error| AnalyzerError::Analysis(error.to_string()))?;
    let value = meter
        .loudness_global()
        .map_err(|error| AnalyzerError::Analysis(error.to_string()))?;
    if !value.is_finite() {
        return Err(AnalyzerError::Analysis(UNMEASURABLE_LOUDNESS.into()));
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sine(amplitude: f32, sample_rate: u32, duration_secs: u32) -> Vec<f32> {
        let sample_count = sample_rate as usize * duration_secs as usize;
        (0..sample_count)
            .map(|index| {
                let time = index as f32 / sample_rate as f32;
                amplitude * (2.0 * std::f32::consts::PI * 1_000.0 * time).sin()
            })
            .collect()
    }

    #[test]
    fn integrated_lufs_matches_the_bs1770_sine_reference() {
        // BS.1770-4: L = -0.691 + 10*log10(mean square) per channel, and the
        // K-weighting filter has +0.6977 dB gain at 1 kHz (evaluated from the
        // standard's stage-1/stage-2 biquad coefficients at 48 kHz). A 1 kHz
        // sine of amplitude A therefore measures
        //   -0.691 + 10*log10(A^2 / 2) + 0.6977  LUFS,
        // which for A = 0.1 is the familiar -23.0 LUFS calibration tone.
        let sample_rate = 48_000;
        let full = integrated_lufs_mono(&sine(1.0, sample_rate, 3), sample_rate).unwrap();
        let half = integrated_lufs_mono(&sine(0.5, sample_rate, 3), sample_rate).unwrap();

        let expected_full = -0.691 + 10.0 * (1.0f64 / 2.0).log10() + 0.6977;
        assert!(
            (full - expected_full).abs() < 0.02,
            "full-scale 1 kHz tone: got {full} LUFS, expected {expected_full}"
        );

        // A linear filter and a fixed gate make every halving of the amplitude
        // drop the reading by exactly 6.0206 dB; any gain, gating or weighting
        // bug shows up here first.
        let drop_db = full - half;
        let expected_drop = 20.0 * 2.0f64.log10();
        assert!(
            (drop_db - expected_drop).abs() < 0.01,
            "halving amplitude should drop {expected_drop:.4} dB, dropped {drop_db}"
        );

        // Anchored to the calibration tone the EBU reference itself names.
        let tone = integrated_lufs_mono(&sine(0.1, sample_rate, 3), sample_rate).unwrap();
        assert!((tone + 23.0).abs() < 0.02, "0.1-amplitude tone: got {tone}");
    }

    #[test]
    fn integrated_lufs_is_finite_and_tracks_relative_level() {
        let sample_rate = 48_000;
        let quiet = integrated_lufs_mono(&sine(0.1, sample_rate, 3), sample_rate).unwrap();
        let loud = integrated_lufs_mono(&sine(0.5, sample_rate, 3), sample_rate).unwrap();

        assert!(quiet.is_finite());
        assert!(loud.is_finite());
        assert!(loud > quiet, "expected {loud} LUFS to exceed {quiet} LUFS");
    }

    #[test]
    fn integrated_lufs_rejects_silence() {
        let result = integrated_lufs_mono(&vec![0.0; 48_000 * 3], 48_000);

        assert!(
            matches!(&result, Err(AnalyzerError::Analysis(message)) if message == UNMEASURABLE_LOUDNESS)
        );
    }
}
