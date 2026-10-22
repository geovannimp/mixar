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

/// Which separation pipeline a built-in model id selects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SeparationModel {
    /// Mixxx HTDemucs waveform model.
    HtDemucs,
    /// SCNet int8 spectrogram model.
    ScNet,
}

/// Map a model id to its separation pipeline.
///
/// Unknown ids fall back to HTDemucs, which is the default offline model.
pub(crate) fn separation_model(model_name: &str) -> SeparationModel {
    match model_name {
        SCNET_MODEL => SeparationModel::ScNet,
        _ => SeparationModel::HtDemucs,
    }
}

/// Separate with a built-in model kind (offline path).
pub fn separate_with_model(
    model_name: &str,
    weights: &std::path::Path,
    pcm: &[f32],
    sample_rate: u32,
    on_progress: Option<&dyn Fn(usize, usize)>,
) -> anyhow::Result<([Vec<f32>; 4], &'static str)> {
    match separation_model(model_name) {
        SeparationModel::ScNet => scnet::separate(weights, pcm, sample_rate, on_progress),
        SeparationModel::HtDemucs => {
            infer::separate_interleaved(weights, pcm, sample_rate, on_progress)
        }
    }
}

pub use manifest::{
    builtin_manifest, cached_model, ensure_from_manifest, ensure_model, resolve_model, Artifact,
    ModelHandle, ModelManifest, DEFAULT_MODEL, SCNET_MODEL, STEM_MODELS,
};

pub use resample_pcm::resample_interleaved_stereo;

pub use split::{split_interleaved_stereo, StemSplitRequest, StemSplitResult, STEM_NAMES};

pub use window::{audio_frame_count, fill_stereo_window};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_selection_routes_to_the_matching_pipeline() {
        assert_eq!(separation_model(DEFAULT_MODEL), SeparationModel::HtDemucs);
        assert_eq!(separation_model(SCNET_MODEL), SeparationModel::ScNet);
        // Unknown ids keep the default HTDemucs pipeline.
        assert_eq!(separation_model("nope"), SeparationModel::HtDemucs);
        assert_eq!(separation_model(""), SeparationModel::HtDemucs);
    }
}
