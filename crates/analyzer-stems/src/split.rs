//! HTDemucs split of interleaved stereo PCM (Burn path).

use std::path::{Path, PathBuf};

use anyhow::{anyhow, Result};
use tracing::info;

use crate::backend::{select_backend, ComputeBackend};
use crate::format::{encode_stem_file, StemAudioFormat};
use crate::manifest::{resolve_model, DEFAULT_MODEL};
use crate::model::SAMPLE_RATE;

/// Canonical stem file order / names.
pub const STEM_NAMES: [&str; 4] = ["vocals", "drums", "bass", "other"];

/// Request to separate already-decoded interleaved stereo PCM.
pub struct StemSplitRequest<'a> {
    pub interleaved_stereo: &'a [f32],
    pub sample_rate: u32,
    pub output_dir: &'a Path,
    /// Mixar-owned model cache root (`{app_support}/models`).
    pub models_root: &'a Path,
    pub model_name: &'a str,
    pub format: StemAudioFormat,
    /// Called with `(windows_done, windows_total)` during Demucs.
    pub on_window_progress: Option<Box<dyn Fn(usize, usize) + Send + Sync + 'a>>,
}

/// Paths to written stem files (same order as [`STEM_NAMES`]).
pub struct StemSplitResult {
    pub paths: [PathBuf; 4],
    pub sample_rate: u32,
    /// `{model id}/{compute backend}`, e.g. `htdemucs_burn_v1/ndarray` or `…/wgpu`.
    pub backend: String,
}

/// Separate interleaved stereo PCM into four stem files under `output_dir`.
pub fn split_interleaved_stereo(req: StemSplitRequest<'_>) -> Result<StemSplitResult> {
    let model_name = if req.model_name.is_empty() {
        DEFAULT_MODEL
    } else {
        req.model_name
    };

    let handle = resolve_model(req.models_root, model_name, None)?;
    let mf = &handle.manifest;

    if mf.sample_rate != SAMPLE_RATE {
        return Err(anyhow!(
            "stem model sample rate must be {SAMPLE_RATE} (got {})",
            mf.sample_rate
        ));
    }

    let progress = req.on_window_progress.as_ref();
    let progress = progress.map(|cb| &**cb as &dyn Fn(usize, usize));

    let backend = select_backend();
    info!(
        model = model_name,
        backend = backend.label(),
        sample_rate = req.sample_rate,
        frames = req.interleaved_stereo.len() / 2,
        "stem split: begin"
    );
    let stems = match backend {
        ComputeBackend::NdArray | ComputeBackend::Wgpu => {
            #[cfg(any(feature = "burn-ndarray", feature = "burn-wgpu"))]
            {
                crate::infer::separate_interleaved(
                    &handle.local_path,
                    req.interleaved_stereo,
                    req.sample_rate,
                    backend,
                    progress,
                )?
            }
            #[cfg(not(any(feature = "burn-ndarray", feature = "burn-wgpu")))]
            {
                let _ = (backend, progress);
                return Err(anyhow!(
                    "analyzer-stems was built without a Burn inference backend"
                ));
            }
        }
    };

    info!(format = ?req.format, "stem split: encoding");
    let mut paths: [PathBuf; 4] = std::array::from_fn(|_| PathBuf::new());
    let mut written_rate = SAMPLE_RATE;
    for ((path, name), stem) in paths.iter_mut().zip(STEM_NAMES).zip(&stems) {
        *path = req
            .output_dir
            .join(format!("{name}.{}", req.format.extension()));
        written_rate = encode_stem_file(path, req.format, stem, SAMPLE_RATE)?;
    }

    Ok(StemSplitResult {
        paths,
        sample_rate: written_rate,
        backend: format!("{model_name}/{}", backend.label()),
    })
}
