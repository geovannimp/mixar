//! Phase 3 deck types still used by AppState (pad mode enum).
//! Pad mode / loop roll commands live on the engine cmd bus.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncMode {
    #[default]
    Off,
    Tempo,
    Beat,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PadMode {
    #[default]
    HotCue,
    LoopRoll,
    BeatJump,
    Sampler,
}
