//! HTDemucs split of interleaved stereo PCM (Burn path; inference stubbed until Task 4).

use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail, Result};

use crate::format::StemAudioFormat;
use crate::manifest::DEFAULT_MODEL;
use crate::resample_pcm::resample_interleaved_stereo;

use crate::manifest::resolve_model;

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

    if mf.sample_rate != 44_100 {
        return Err(anyhow!(
            "stem model sample rate must be 44100 (got {})",
            mf.sample_rate
        ));
    }

    let _samples = if req.sample_rate == mf.sample_rate {
        req.interleaved_stereo.to_vec()
    } else {
        resample_interleaved_stereo(req.interleaved_stereo, req.sample_rate, mf.sample_rate)?
    };

    let _ = (&handle, req.output_dir, req.format, req.on_window_progress);
    bail!("Burn HTDemucs not wired yet")
}
