use serde::{Deserialize, Serialize};

/// Message source on the engine bus.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Origin {
    Engine,
    Mixer,
    Deck(u16),
}
