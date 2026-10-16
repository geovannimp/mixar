use serde::{Deserialize, Serialize};

/// Shared action/event discriminator; cmd vs evt is determined by which bus carries the message.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    AnalyzeTrack,
    /// Standalone stem generation for one track. Independent of `AnalyzeTrack`.
    GenerateStems,
    TrackAnalyzed,
    RefreshTrack,
    TrackUpdated,
    SaveHotCue,
    DeleteHotCue,
    SaveLoop,
    DeleteLoop,
    SaveBeatGrid,
    HotCuesChanged,
    LoopsChanged,
    BeatGridChanged,
    HistorySessionUpdated,
    Navigate,
    /// UI-only: move browser focus to the next / previous pane (sidebar ↔ list).
    Focus,
    Load,
    Error,
    Notice,
    /// Coarse per-track job progress (analysis / stems phases).
    TrackProgress,
}
