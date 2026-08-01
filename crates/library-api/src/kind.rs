use serde::{Deserialize, Serialize};

/// Shared action/event discriminator; cmd vs evt is determined by which bus carries the message.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    AnalyzeTrack,
    TrackAnalyzed,
    RefreshTrack,
    TrackUpdated,
    SaveHotCue,
    DeleteHotCue,
    SaveLoop,
    DeleteLoop,
    HotCuesChanged,
    LoopsChanged,
    Error,
    Notice,
}
