//! Offline stem separation for Mixar.
//!
//! Mixar-owned Burn HTDemucs on already-decoded interleaved stereo PCM
//! (no second file decode).

mod backend;
pub mod dsp;
mod format;
#[cfg(any(feature = "burn-ndarray", feature = "burn-wgpu"))]
mod infer;
mod manifest;
pub mod model;
mod resample_pcm;
mod split;
pub mod weights;
mod window;

/// CPU backend used for headless / CI inference.
#[cfg(feature = "burn-ndarray")]
pub type NdArrayBackend = burn::backend::NdArray<f32>;

/// GPU backend (Vulkan/Metal/DX12 via Burn wgpu).
#[cfg(feature = "burn-wgpu")]
pub type WgpuBackend = burn::backend::Wgpu<f32, i32>;

pub use backend::{select_backend, ComputeBackend};

pub use dsp::{cac_planar_to_complex, stft_to_cac_planar, Stft, HOP_LENGTH, N_FFT};

pub use model::{
    HTDemucs, CHANNELS, DEPTH, GROWTH, KERNEL_SIZE, SAMPLE_RATE, STRIDE, TRAINING_LENGTH, T_HEADS,
    T_LAYERS,
};

#[cfg(any(feature = "burn-ndarray", feature = "burn-wgpu"))]
pub use infer::separate_interleaved;

pub use weights::{load_htdemucs_from_safetensors, TensorStore, HTDEMUCS_SIGNATURE};

pub use format::{encode_stem_file, StemAudioFormat, OPUS_BITRATE_BPS, OPUS_SAMPLE_RATE};

pub use manifest::{
    ensure_from_manifest, ensure_model, resolve_model, Artifact, ModelHandle, ModelManifest,
    DEFAULT_MODEL,
};

pub use resample_pcm::resample_interleaved_stereo;

pub use split::{split_interleaved_stereo, StemSplitRequest, StemSplitResult, STEM_NAMES};

pub use window::{audio_frame_count, fill_stereo_window};
