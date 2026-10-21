//! Offline and live stem separation for Mixar.
//!
//! Mixar-owned ort Session + Mixxx HTDemucs ONNX on already-decoded
//! interleaved stereo PCM (no second file decode).

mod ep;
mod format;
mod infer;
mod live;
mod manifest;
mod resample_pcm;
mod session;
mod split;
mod window;

pub use ep::{
    ep_label, force_ep_from_env, preferred_ep_labels, preferred_execution_providers, FORCE_EP_ENV,
};

pub use format::{encode_stem_file, StemAudioFormat, OPUS_BITRATE_BPS, OPUS_SAMPLE_RATE};

pub use infer::{separate_interleaved, SAMPLE_RATE as STEM_SAMPLE_RATE};

pub use live::{start_live_separator, LiveSeparator};

pub use manifest::{
    builtin_manifest, cached_model, ensure_from_manifest, ensure_model, resolve_model, Artifact,
    ModelHandle, ModelManifest, DEFAULT_MODEL,
};

pub use resample_pcm::resample_interleaved_stereo;

pub use split::{split_interleaved_stereo, StemSplitRequest, StemSplitResult, STEM_NAMES};

/// Load the process-global ORT session ahead of the first separation.
///
/// Returns the execution-provider label that won the cascade. Safe to call on
/// a background thread; the session is shared by offline and live separation.
pub fn preload_session(weights: &std::path::Path) -> anyhow::Result<&'static str> {
    session::with_session(weights, |_, ep, _, _| Ok(ep))
}

pub use window::{audio_frame_count, fill_stereo_window};
