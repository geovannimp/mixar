use serde::{Deserialize, Serialize};

/// Message source on the library bus.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Origin {
    Library,
    Track(String),
}
