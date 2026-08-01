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

/// Persisted hot cue row for bus payloads.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HotCue {
    pub slot: u8,
    pub position_ms: i32,
    pub loop_length_beats: Option<i32>,
    pub color: Option<String>,
    pub label: Option<String>,
}

/// Persisted saved-loop row for bus payloads.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SavedLoop {
    pub slot: u8,
    pub in_ms: i32,
    pub out_ms: i32,
    pub label: Option<String>,
    pub color: Option<String>,
}

/// Nested command body on the library cmd bus.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CmdBody {
    Empty,
    AnalyzeTrack {
        track_id: String,
        force: bool,
    },
    SaveHotCue {
        track_id: String,
        slot: u8,
        position_ms: i32,
        #[serde(default)]
        loop_length_beats: Option<i32>,
        #[serde(default)]
        color: Option<String>,
        #[serde(default)]
        label: Option<String>,
    },
    DeleteHotCue {
        track_id: String,
        slot: u8,
    },
    SaveLoop {
        track_id: String,
        slot: u8,
        in_ms: i32,
        out_ms: i32,
        #[serde(default)]
        label: Option<String>,
        #[serde(default)]
        color: Option<String>,
    },
    DeleteLoop {
        track_id: String,
        slot: u8,
    },
}

/// Nested event body on the library evt bus.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EvtBody {
    Empty,
    TrackAnalyzed {
        track: TrackSummary,
    },
    HotCuesChanged {
        track_id: String,
        hot_cues: Vec<HotCue>,
    },
    LoopsChanged {
        track_id: String,
        loops: Vec<SavedLoop>,
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
