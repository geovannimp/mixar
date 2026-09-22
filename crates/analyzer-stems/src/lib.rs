//! Offline stem separation for Mixar.
//!
//! Uses [stem-splitter-core](https://crates.io/crates/stem-splitter-core) HTDemucs ONNX
//! on already-decoded interleaved stereo PCM (no second file decode).
//! Mixar-owned [`manifest`] downloads StemSplit `htdemucs_ort_v2` for the ORT pivot.

mod ep;
mod format;
mod manifest;
mod ort_ep;
mod split;
mod window;

pub use ep::{
    ep_label, force_ep_from_env, preferred_ep_labels, preferred_execution_providers, FORCE_EP_ENV,
};

pub use format::{encode_stem_file, StemAudioFormat, OPUS_BITRATE_BPS, OPUS_SAMPLE_RATE};

// Mixar ort_v2 manifest (wired in Task 3). Keep SSC ensure/DEFAULT_MODEL public until then.
pub use manifest::{builtin_manifest, Artifact};

pub use ort_ep::prepare_ort_execution_providers;

pub use split::{
    ensure_from_manifest, ensure_model, resolve_model, split_interleaved_stereo, StemSplitRequest,
    StemSplitResult, DEFAULT_MODEL, STEM_NAMES,
};

pub use window::{audio_frame_count, fill_stereo_window};
