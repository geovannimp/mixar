//! Offline stem separation for Mixar.
//!
//! Mixar-owned Burn HTDemucs on already-decoded interleaved stereo PCM
//! (no second file decode).

mod format;
mod manifest;
mod resample_pcm;
mod split;
mod window;

pub use format::{encode_stem_file, StemAudioFormat, OPUS_BITRATE_BPS, OPUS_SAMPLE_RATE};

pub use manifest::{
    ensure_from_manifest, ensure_model, resolve_model, Artifact, ModelHandle, ModelManifest,
    DEFAULT_MODEL,
};

pub use resample_pcm::resample_interleaved_stereo;

pub use split::{split_interleaved_stereo, StemSplitRequest, StemSplitResult, STEM_NAMES};

pub use window::{audio_frame_count, fill_stereo_window};
