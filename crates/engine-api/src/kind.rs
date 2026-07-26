use serde::{Deserialize, Serialize};

/// Shared action/event discriminator; cmd vs evt is determined by which bus carries the message.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Kind {
    Play,
    Pause,
    Seek,
    SetVolume,
    SetEq,
    SetSpeed,
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
