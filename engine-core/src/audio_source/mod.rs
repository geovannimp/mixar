//! Audio loading from arbitrary sources (disk, network, memory, etc.).

mod file;
mod source;

pub use file::FileAudioSource;
pub use source::{AudioSource, LoadedAudio};
