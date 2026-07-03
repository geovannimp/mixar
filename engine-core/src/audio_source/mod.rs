//! Audio loading from arbitrary sources (disk, network, memory, etc.).

mod file;

pub use audio_core::{AudioSource, LoadedAudio};
pub use file::FileAudioSource;
