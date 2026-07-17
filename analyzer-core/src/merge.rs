use crate::result::TrackAnalysis;

/// Minimal tag-side metadata used when merging analysis into library fields.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TagMetadata {
    pub bpm: Option<f64>,
    pub key: Option<String>,
}

/// Merge file tags with analysis results according to force/confidence policy.
pub fn merge_track_metadata(
    tags: &TagMetadata,
    analysis: &TrackAnalysis,
    force: bool,
    min_bpm_confidence: f32,
    min_key_confidence: f32,
) -> TagMetadata {
    let mut out = tags.clone();

    if let Some(bpm) = &analysis.bpm {
        let use_analysis = force || out.bpm.is_none() || bpm.confidence >= min_bpm_confidence;
        if use_analysis {
            out.bpm = Some(bpm.bpm);
        }
    }

    if let Some(key) = &analysis.key {
        let use_analysis = force || out.key.is_none() || key.confidence >= min_key_confidence;
        if use_analysis {
            out.key = Some(key.musical.clone());
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::result::{AnalysisRunMetadata, BpmAnalysis, KeyAnalysis};

    fn sample_analysis(bpm: f64, bpm_conf: f32, key: &str, key_conf: f32) -> TrackAnalysis {
        TrackAnalysis {
            bpm: Some(BpmAnalysis {
                bpm,
                confidence: bpm_conf,
            }),
            key: Some(KeyAnalysis {
                musical: key.to_string(),
                confidence: key_conf,
                clarity: 0.9,
            }),
            beat_grid: None,
            loudness_lufs: None,
            metadata: AnalysisRunMetadata {
                backend: "test".into(),
                backend_version: "0".into(),
                analyzed_at: "2026-01-01T00:00:00Z".into(),
                sample_rate: 44100,
                duration_analyzed_secs: 1.0,
            },
        }
    }

    #[test]
    fn force_overrides_tags() {
        let tags = TagMetadata {
            bpm: Some(120.0),
            key: Some("Am".into()),
        };
        let analysis = sample_analysis(128.0, 0.2, "F#m", 0.2);
        let merged = merge_track_metadata(&tags, &analysis, true, 0.5, 0.5);
        assert_eq!(merged.bpm, Some(128.0));
        assert_eq!(merged.key.as_deref(), Some("F#m"));
    }

    #[test]
    fn keeps_tags_when_confidence_low_and_not_forced() {
        let tags = TagMetadata {
            bpm: Some(120.0),
            key: Some("Am".into()),
        };
        let analysis = sample_analysis(128.0, 0.2, "F#m", 0.2);
        let merged = merge_track_metadata(&tags, &analysis, false, 0.5, 0.5);
        assert_eq!(merged.bpm, Some(120.0));
        assert_eq!(merged.key.as_deref(), Some("Am"));
    }

    #[test]
    fn fills_missing_from_analysis() {
        let tags = TagMetadata::default();
        let analysis = sample_analysis(128.0, 0.9, "F#m", 0.9);
        let merged = merge_track_metadata(&tags, &analysis, false, 0.5, 0.5);
        assert_eq!(merged.bpm, Some(128.0));
        assert_eq!(merged.key.as_deref(), Some("F#m"));
    }
}
