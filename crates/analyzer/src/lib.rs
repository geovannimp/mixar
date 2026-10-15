//! Offline audio analysis: decode files to mono PCM and run the qm-dsp backend.

mod loudness;
mod preprocess;

pub mod decode;

use std::path::Path;
use std::sync::OnceLock;

pub use analyzer_core::{
    merge_track_metadata, snap_grid, AnalysisConfig, AnalysisRunMetadata, AnalysisTargets,
    AnalyzerError, AudioAnalyzer, BeatGridAnalysis, BpmAnalysis, KeyAnalysis, Result, SnapConfig,
    TagMetadata, TrackAnalysis,
};
pub use analyzer_qmdsp::QmdspAnalyzer;
pub use decode::{decode_mono, DecodedMono};
pub use loudness::integrated_lufs_mono;

static DEFAULT_ANALYZER: OnceLock<QmdspAnalyzer> = OnceLock::new();

fn default_analyzer() -> &'static QmdspAnalyzer {
    DEFAULT_ANALYZER.get_or_init(QmdspAnalyzer::new)
}

/// Analyze already-decoded mono PCM, normalizing it to ±1.0 first.
pub fn analyze_pcm(
    samples: &[f32],
    sample_rate: u32,
    config: &AnalysisConfig,
) -> Result<TrackAnalysis> {
    analyze_prepared(preprocess::prepare(samples, sample_rate, config), config)
}

/// Decode a file and run analysis with the default backend.
pub fn analyze_file(path: &Path, config: &AnalysisConfig) -> Result<TrackAnalysis> {
    let decoded =
        decode::decode_mono(path, config).map_err(|e| AnalyzerError::Decode(e.to_string()))?;
    // The decoder hands back one mono buffer and `prepare_owned` rescales it in
    // place when it is out of range, so there is no decoded copy plus a
    // normalized copy.
    analyze_prepared(preprocess::prepare_owned(decoded), config)
}

/// Run the backend, measure loudness and snap the grid on prepared PCM.
fn analyze_prepared(
    processed: preprocess::PreparedPcm<'_>,
    config: &AnalysisConfig,
) -> Result<TrackAnalysis> {
    let samples = &processed.samples;
    let sample_rate = processed.sample_rate;
    let mut track = default_analyzer().analyze_pcm(samples, sample_rate, config)?;
    track.loudness_lufs = Some(loudness_lufs_or_quiet(samples, sample_rate)?);
    snap_grid(&mut track, &config.snap);
    Ok(track)
}

/// shortcut: EBU R128 returns non-finite for silence / sub-gate clips. Map to a
/// very-quiet LUFS so deck load ensure can finish; auto-gain clamps the boost.
const UNMEASURABLE_LOUDNESS_LUFS: f64 = -70.0;

fn loudness_lufs_or_quiet(samples: &[f32], sample_rate: u32) -> Result<f64> {
    match integrated_lufs_mono(samples, sample_rate) {
        Ok(value) => Ok(value),
        Err(AnalyzerError::Analysis(message)) if message == loudness::UNMEASURABLE_LOUDNESS => {
            Ok(UNMEASURABLE_LOUDNESS_LUFS)
        }
        Err(error) => Err(error),
    }
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

    /// Write a stereo WAV whose samples swing past full scale.
    ///
    /// An i16 container cannot do this: S16→f32 is `sample / 32768.0`, so the
    /// decoded peak is capped at 32767/32768 no matter what is written. The
    /// float container carries the values verbatim (`codec`'s
    /// `AudioBufferRef::F32` arm copies them through unchanged), which is the
    /// only way a decoded track can exceed ±1.0 and put the peak-normalize
    /// branch on the path.
    fn write_hot_stereo_wav(path: &std::path::Path, frames: u32) {
        let spec = hound::WavSpec {
            channels: 2,
            sample_rate: 44100,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        };
        let mut writer = hound::WavWriter::create(path, spec).unwrap();
        for i in 0..frames {
            let t = i as f32 / 44100.0;
            let beat = (2.0 * std::f32::consts::PI * 2.0 * t).sin();
            let left = 1.8 * beat * (2.0 * std::f32::consts::PI * 440.0 * t).sin();
            let right = 1.7 * beat * (2.0 * std::f32::consts::PI * 660.0 * t).sin();
            writer.write_sample(left).unwrap();
            writer.write_sample(right).unwrap();
        }
        writer.finalize().unwrap();
    }

    fn without_timestamp(mut track: TrackAnalysis) -> TrackAnalysis {
        track.metadata.analyzed_at = String::new();
        track
    }

    #[test]
    fn analyze_file_runs_on_wav() {
        let dir = tempfile::tempdir().unwrap();
        let wav = dir.path().join("tone.wav");
        write_minimal_wav(&wav);

        let config = AnalysisConfig {
            max_duration_ms: Some(5000),
            ..Default::default()
        };

        let result = analyze_file(&wav, &config).expect("analysis should succeed");
        assert!(
            result.loudness_lufs.is_some_and(f64::is_finite),
            "analysis should include finite integrated loudness"
        );
    }

    #[test]
    fn hot_stereo_track_reports_a_sane_analysis() {
        let dir = tempfile::tempdir().unwrap();
        let wav = dir.path().join("hot.wav");
        write_hot_stereo_wav(&wav, 44_100 * 4);

        let config = AnalysisConfig::default();
        // The fixture must decode past full scale, or this test and the
        // equivalence test below never run the peak-normalize branch.
        let decoded = decode::decode_mono(&wav, &config).unwrap();
        assert!(
            decoded.samples.iter().any(|sample| sample.abs() > 1.0),
            "the hot fixture must decode past full scale, or normalization is untested"
        );
        let analysis = analyze_file(&wav, &config).expect("analysis should succeed");

        let bpm = analysis.bpm.expect("a 2 Hz click is a detectable tempo");
        assert!(
            (bpm.bpm - 120.0).abs() < 2.0,
            "expected ~120 BPM, got {}",
            bpm.bpm
        );
        let grid = analysis.beat_grid.expect("beat grid should be present");
        assert!(grid.beats.len() > 4, "got only {} beats", grid.beats.len());
        // Peak normalization must have brought the buffer inside ±1.0, so the
        // measured loudness tracks the tone rather than clipping.
        let loudness = analysis.loudness_lufs.expect("loudness should be set");
        assert!(loudness.is_finite());
    }

    /// The owned path (`analyze_file` → `prepare_owned`, rescaling the decoded
    /// buffer in place) and the borrowed path (`analyze_pcm` → `prepare`,
    /// collecting through the scale factor) must land on identical analysis.
    /// The fixture is hot, so both take the rescale branch and the two scale
    /// implementations are compared rather than two no-ops.
    #[test]
    fn owned_and_borrowed_prepare_paths_agree() {
        let dir = tempfile::tempdir().unwrap();
        let wav = dir.path().join("hot.wav");
        write_hot_stereo_wav(&wav, 44_100 * 4);

        let config = AnalysisConfig::default();
        let fused = analyze_file(&wav, &config).expect("analysis should succeed");

        let decoded = decode::decode_mono(&wav, &config).unwrap();
        let reference = analyze_pcm(&decoded.samples, decoded.sample_rate, &config)
            .expect("analysis should succeed");

        assert_eq!(
            without_timestamp(fused),
            without_timestamp(reference),
            "the owned and borrowed prepare paths disagree"
        );
    }

    /// Silence must still load: the unmeasurable-loudness sentinel has to fire,
    /// and the grid work must not depend on a finite loudness.
    #[test]
    fn silent_track_still_loads_with_the_quiet_sentinel() {
        let dir = tempfile::tempdir().unwrap();
        let wav = dir.path().join("silence.wav");
        write_minimal_wav(&wav);
        // Rewrite it as pure silence.
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: 44100,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::create(&wav, spec).unwrap();
        for _ in 0..44100 * 2 {
            writer.write_sample(0i16).unwrap();
        }
        writer.finalize().unwrap();

        let analysis = analyze_file(&wav, &AnalysisConfig::default()).unwrap();
        assert_eq!(analysis.loudness_lufs, Some(UNMEASURABLE_LOUDNESS_LUFS));
    }

    /// A hot mono PCM buffer is what `analyze_pcm` receives from callers that
    /// decoded it themselves; normalization must bring it inside ±1.0.
    #[test]
    fn analyze_pcm_normalizes_out_of_range_input() {
        let sample_rate = 48_000u32;
        let mut samples = Vec::with_capacity(sample_rate as usize * 2);
        for index in 0..sample_rate as usize * 2 {
            let t = index as f32 / sample_rate as f32;
            samples.push(4.0 * (2.0 * std::f32::consts::PI * 2.0 * t).sin());
        }

        let analysis = analyze_pcm(&samples, sample_rate, &AnalysisConfig::default()).unwrap();

        // A 4.0-peak sine scaled to 1.0: same shape as the 1 kHz reference tone,
        // 2 beats per second, so ~120 BPM.
        let bpm = analysis.bpm.expect("2 Hz click is a detectable tempo");
        assert!((bpm.bpm - 120.0).abs() < 3.0, "got {} BPM", bpm.bpm);
        assert!(analysis.loudness_lufs.is_some_and(f64::is_finite));
    }
}
