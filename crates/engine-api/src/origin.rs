use serde::{Deserialize, Serialize};

/// Message source on the engine bus.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Origin {
    Engine,
    Mixer,
    Deck(u16),
}
