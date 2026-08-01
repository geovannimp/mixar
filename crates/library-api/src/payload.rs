use serde::{Deserialize, Serialize};

/// Library track summary for host/UI (mirrors GUI `TrackSummary`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TrackSummary {
    pub id: String,
    pub display_name: String,
    pub artist: Option<String>,
    pub title: Option<String>,
    pub album: Option<String>,
    pub genre: Option<String>,
    pub bpm: Option<f64>,
    pub key: Option<String>,
    pub duration_ms: Option<i32>,
    pub path: String,
}

/// Nested command body on the library cmd bus.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CmdBody {
    Empty,
    AnalyzeTrack { track_id: String, force: bool },
}

/// Nested event body on the library evt bus.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EvtBody {
    Empty,
    TrackAnalyzed {
        track: TrackSummary,
    },
    Error {
        message: String,
        #[serde(default)]
        track_id: Option<String>,
    },
    Notice {
        message: String,
    },
}
