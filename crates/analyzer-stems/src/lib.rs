//! Offline stem separation for Mixar.
//!
//! Mixar-owned ort Session + Mixxx HTDemucs ONNX on already-decoded
//! interleaved stereo PCM (no second file decode).

mod ep;
mod format;
mod infer;
mod manifest;
mod resample_pcm;
mod scnet;
mod session;
mod split;
mod window;

pub use ep::{
    ep_label, force_ep_from_env, preferred_ep_labels, preferred_execution_providers, FORCE_EP_ENV,
};

pub use format::{encode_stem_file, StemAudioFormat, OPUS_BITRATE_BPS, OPUS_SAMPLE_RATE};

pub use infer::{separate_interleaved, SAMPLE_RATE as STEM_SAMPLE_RATE};

/// Separate with a built-in model kind (offline path).
pub fn separate_with_model(
    model_name: &str,
    weights: &std::path::Path,
    pcm: &[f32],
    sample_rate: u32,
    on_progress: Option<&dyn Fn(usize, usize)>,
) -> anyhow::Result<([Vec<f32>; 4], &'static str)> {
    if model_name == SCNET_MODEL {
        scnet::separate(weights, pcm, sample_rate, on_progress)
    } else {
        infer::separate_interleaved(weights, pcm, sample_rate, on_progress)
    }
}

pub use manifest::{
    builtin_manifest, cached_model, ensure_from_manifest, ensure_model, resolve_model, Artifact,
    ModelHandle, ModelManifest, DEFAULT_MODEL, SCNET_MODEL, STEM_MODELS,
};

pub use resample_pcm::resample_interleaved_stereo;

pub use split::{split_interleaved_stereo, StemSplitRequest, StemSplitResult, STEM_NAMES};

pub use window::{audio_frame_count, fill_stereo_window};
