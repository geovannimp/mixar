use analyzer_core::AnalyzerError;
use ebur128::{EbuR128, Mode};

/// Measure integrated EBU R128 loudness for mono PCM.
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
        return Err(AnalyzerError::Analysis("non-finite loudness".into()));
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
            matches!(result, Err(AnalyzerError::Analysis(message)) if message == "non-finite loudness")
        );
    }
}
