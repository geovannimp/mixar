//! Offline stem separation for Mixar.
//!
//! Uses [stem-splitter-core](https://crates.io/crates/stem-splitter-core) HTDemucs ONNX
//! on already-decoded interleaved stereo PCM (no second file decode).

mod split;
mod window;

pub use split::{
    ensure_from_manifest, ensure_model, resolve_model, split_interleaved_stereo, StemSplitRequest,
    StemSplitResult, DEFAULT_MODEL, STEM_NAMES,
};

pub use window::{audio_frame_count, fill_stereo_window};
