use crate::Sample;
use anyhow::Result;

/// Decoded audio ready to load into a deck.
#[derive(Debug, Clone)]
pub struct LoadedAudio {
    pub samples: Vec<Sample>,
    pub sample_rate: u32,
    pub channels: u16,
    /// Identifier for the source (path, URL, etc.) used for deck display and logging.
    pub source_id: String,
}

/// Interface for decoding audio from any origin into [`LoadedAudio`].
pub trait LoadableAudio {
    /// Load and decode audio from this source.
    fn load(&self) -> Result<LoadedAudio>;
}

impl LoadableAudio for LoadedAudio {
    fn load(&self) -> Result<LoadedAudio> {
        Ok(self.clone())
    }
}
