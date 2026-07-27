use serde::{Deserialize, Serialize};

/// Shared action/event discriminator; cmd vs evt is determined by which bus carries the message.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Play,
    Pause,
    Seek,
    SetVolume,
    SetEq,
    SetSpeed,
    SetFilter,
    SetGainTrim,
    SetHeadphoneCue,
    SetCrossfader,
    SetCueMix,
    SetMasterCue,
    Updated,
    Position,
    Levels,
    Status,
    Error,
    Notice,
}
