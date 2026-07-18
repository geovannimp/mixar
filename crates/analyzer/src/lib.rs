//! Offline audio analysis: decode files to mono PCM and run the default backend.

mod decode;
mod loudness;
mod preprocess;

use std::path::Path;
use std::sync::OnceLock;

pub use analyzer_core::{
    merge_track_metadata, AnalysisConfig, AnalysisRunMetadata, AnalysisTargets, AnalyzerError,
    AudioAnalyzer, BeatGridAnalysis, BpmAnalysis, KeyAnalysis, Result, TagMetadata, TrackAnalysis,
};
pub use analyzer_stratum::{musical_key_from_stratum, StratumAnalyzer};
pub use loudness::integrated_lufs_mono;

static DEFAULT_ANALYZER: OnceLock<analyzer_stratum::StratumAnalyzer> = OnceLock::new();

fn default_analyzer() -> &'static analyzer_stratum::StratumAnalyzer {
    DEFAULT_ANALYZER.get_or_init(analyzer_stratum::StratumAnalyzer::new)
}

/// Analyze already-decoded mono PCM (normalized ±1.0).
pub fn analyze_pcm(
    samples: &[f32],
    sample_rate: u32,
    config: &AnalysisConfig,
) -> Result<TrackAnalysis> {
    let processed = preprocess::prepare(samples, sample_rate, config);
    let mut track =
        default_analyzer().analyze_pcm(&processed.samples, processed.sample_rate, config)?;
    track.loudness_lufs = Some(integrated_lufs_mono(
        &processed.samples,
        processed.sample_rate,
    )?);
    Ok(track)
}

/// Decode a file and run analysis with the default backend.
pub fn analyze_file(path: &Path, config: &AnalysisConfig) -> Result<TrackAnalysis> {
    let decoded =
        decode::decode_mono(path, config).map_err(|e| AnalyzerError::Decode(e.to_string()))?;
    analyze_pcm(&decoded.samples, decoded.sample_rate, config)
}

/// Analyze with a custom backend (testing or alternate backends).
pub fn analyze_pcm_with<A: AudioAnalyzer>(
    analyzer: &A,
    samples: &[f32],
    sample_rate: u32,
    config: &AnalysisConfig,
) -> Result<TrackAnalysis> {
    let processed = preprocess::prepare(samples, sample_rate, config);
    let mut track = analyzer.analyze_pcm(&processed.samples, processed.sample_rate, config)?;
    track.loudness_lufs = Some(integrated_lufs_mono(
        &processed.samples,
        processed.sample_rate,
    )?);
    Ok(track)
}

/// Decode a file and analyze with a custom backend.
pub fn analyze_file_with<A: AudioAnalyzer>(
    analyzer: &A,
    path: &Path,
    config: &AnalysisConfig,
) -> Result<TrackAnalysis> {
    let decoded =
        decode::decode_mono(path, config).map_err(|e| AnalyzerError::Decode(e.to_string()))?;
    analyze_pcm_with(analyzer, &decoded.samples, decoded.sample_rate, config)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_minimal_wav(path: &std::path::Path) {
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: 44100,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::create(path, spec).unwrap();
        for i in 0..44100 {
            let t = i as f32 / 44100.0;
            let sample =
                (0.3 * (2.0 * std::f32::consts::PI * 440.0 * t).sin() * i16::MAX as f32) as i16;
            writer.write_sample(sample).unwrap();
        }
        writer.finalize().unwrap();
    }

    #[test]
    fn analyze_file_runs_on_wav() {
        let dir = tempfile::tempdir().unwrap();
        let wav = dir.path().join("tone.wav");
        write_minimal_wav(&wav);

        let config = AnalysisConfig {
            max_duration_secs: Some(5.0),
            ..Default::default()
        };

        let result = analyze_file(&wav, &config).expect("analysis should succeed");
        assert!(
            result.loudness_lufs.is_some_and(f64::is_finite),
            "analysis should include finite integrated loudness"
        );
    }
}
