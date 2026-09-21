//! HTDemucs split of interleaved stereo PCM via stem-splitter-core.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use stem_splitter_core::core::engine;
use stem_splitter_core::io::crypto::verify_sha256;
use stem_splitter_core::io::net::{download_with_progress, http_client};
use stem_splitter_core::model::registry::resolve_manifest_url;
use stem_splitter_core::{ModelHandle, ModelManifest};

use crate::format::{encode_stem_file, StemAudioFormat};
use crate::window::{audio_frame_count, fill_stereo_window};

/// Default ONNX model id (stem-splitter-core registry).
pub const DEFAULT_MODEL: &str = "htdemucs_ort_v1";

/// Canonical stem file order / names.
pub const STEM_NAMES: [&str; 4] = ["vocals", "drums", "bass", "other"];

/// Request to separate already-decoded interleaved stereo PCM.
pub struct StemSplitRequest<'a> {
    pub interleaved_stereo: &'a [f32],
    pub sample_rate: u32,
    pub output_dir: &'a Path,
    /// Mixar-owned ONNX cache root (`{app_support}/models`).
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

/// Download / verify model weights into `models_root` and preload the session.
pub fn ensure_model(models_root: &Path, model_name: &str) -> Result<()> {
    crate::ort_ep::prepare_ort_execution_providers();
    let handle = resolve_model(models_root, model_name, None)?;
    engine::preload(&handle).map_err(ssc_err)?;
    Ok(())
}

/// Resolve (download if needed) a model into `models_root` without preloading.
pub fn resolve_model(
    models_root: &Path,
    model_name: &str,
    manifest_url_override: Option<&str>,
) -> Result<ModelHandle> {
    let manifest_url = match manifest_url_override {
        Some(url) => url.to_string(),
        None => resolve_manifest_url(model_name).map_err(ssc_err)?,
    };

    let client = http_client();
    let manifest: ModelManifest = client
        .get(&manifest_url)
        .send()
        .map_err(ssc_err)?
        .error_for_status()
        .map_err(ssc_err)?
        .json()
        .map_err(ssc_err)?;

    ensure_from_manifest(models_root, &manifest)
}

/// Place / reuse the primary artifact under `models_root` from an already-fetched manifest.
pub fn ensure_from_manifest(models_root: &Path, manifest: &ModelManifest) -> Result<ModelHandle> {
    let artifact = manifest
        .resolve_primary_artifact()
        .map_err(|msg| anyhow!("stem model manifest: {msg}"))?;

    std::fs::create_dir_all(models_root)
        .with_context(|| format!("create models root {}", models_root.display()))?;

    let local_path = artifact_path(models_root, manifest, &artifact.file, &artifact.sha256)?;

    let need_download = !matches!(verify_sha256(&local_path, &artifact.sha256), Ok(true));
    if need_download {
        let client = http_client();
        download_with_progress(&client, &artifact.url, &local_path).map_err(ssc_err)?;
        if !verify_sha256(&local_path, &artifact.sha256).map_err(ssc_err)? {
            return Err(anyhow!(
                "checksum mismatch for stem model {}",
                local_path.display()
            ));
        }
        if artifact.size_bytes > 0 {
            let size = std::fs::metadata(&local_path).map(|m| m.len()).unwrap_or(0);
            if size != artifact.size_bytes {
                eprintln!(
                    "stem model size mismatch for {}, expected {} bytes, got {}",
                    local_path.display(),
                    artifact.size_bytes,
                    size
                );
            }
        }
    }

    Ok(ModelHandle {
        manifest: manifest.clone(),
        local_path,
    })
}

fn artifact_path(
    models_root: &Path,
    manifest: &ModelManifest,
    file: &str,
    sha256: &str,
) -> Result<PathBuf> {
    let name = safe_path_component(&manifest.name).context("unsafe model manifest name")?;
    let ext = safe_artifact_ext(file)?;
    let short = safe_sha_prefix(sha256)?;
    let file_name = format!("{name}-{short}{ext}");
    let path = models_root.join(&file_name);
    // Defense in depth: keep the resolved path under models_root.
    if path
        .file_name()
        .and_then(|s| s.to_str())
        .is_none_or(|s| s != file_name)
        || path.parent() != Some(models_root)
    {
        return Err(anyhow!(
            "stem model path escapes models_root: {}",
            path.display()
        ));
    }
    Ok(path)
}

/// Reject absolute / traversal / multi-component names from untrusted manifests.
fn safe_path_component(s: &str) -> Result<&str> {
    if s.is_empty() || s == "." || s == ".." {
        return Err(anyhow!("empty or reserved path component"));
    }
    if s.contains(['/', '\\', '\0']) {
        return Err(anyhow!("path separators not allowed"));
    }
    let mut comps = Path::new(s).components();
    match (comps.next(), comps.next()) {
        (Some(std::path::Component::Normal(os)), None) if os == s => Ok(s),
        _ => Err(anyhow!("unsafe path component '{s}'")),
    }
}

fn safe_artifact_ext(file: &str) -> Result<String> {
    if file.is_empty() {
        return Err(anyhow!("empty artifact file name"));
    }
    for c in Path::new(file).components() {
        if !matches!(c, std::path::Component::Normal(_)) {
            return Err(anyhow!("unsafe artifact file path '{file}'"));
        }
    }
    let base = Path::new(file)
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or_else(|| anyhow!("unsafe artifact file name"))?;
    let base = safe_path_component(base)?;
    match base.rsplit_once('.') {
        Some((_, ext)) if !ext.is_empty() && !ext.contains(['/', '\\']) => Ok(format!(".{ext}")),
        _ => Ok(String::new()),
    }
}

fn safe_sha_prefix(sha256: &str) -> Result<&str> {
    if sha256.len() < 8 || !sha256.bytes().take(8).all(|b| b.is_ascii_hexdigit()) {
        return Err(anyhow!("invalid artifact sha256 prefix"));
    }
    Ok(&sha256[..8])
}

/// Separate interleaved stereo PCM into four stem files under `output_dir`.
pub fn split_interleaved_stereo(req: StemSplitRequest<'_>) -> Result<StemSplitResult> {
    let model_name = if req.model_name.is_empty() {
        DEFAULT_MODEL
    } else {
        req.model_name
    };

    let handle = resolve_model(req.models_root, model_name, None)?;
    crate::ort_ep::prepare_ort_execution_providers();
    engine::preload(&handle).map_err(ssc_err)?;
    let mf = engine::manifest();

    if mf.sample_rate != 44_100 {
        return Err(anyhow!(
            "stem model sample rate must be 44100 (got {})",
            mf.sample_rate
        ));
    }

    let samples = if req.sample_rate == mf.sample_rate {
        req.interleaved_stereo.to_vec()
    } else {
        // ponytail: linear resample ceiling — upgrade to rubato FFT when stem quality audits need it.
        resample_interleaved_linear(req.interleaved_stereo, req.sample_rate, mf.sample_rate)
    };

    let n = audio_frame_count(&samples, 2);
    if n == 0 {
        return Err(anyhow!("empty audio"));
    }

    let win = mf.window;
    let hop = mf.hop;
    if !(win > 0 && hop > 0 && hop <= win) {
        return Err(anyhow!("bad win/hop in model manifest"));
    }

    let names = if mf.stems.is_empty() {
        STEM_NAMES.map(str::to_string).to_vec()
    } else {
        mf.stems.clone()
    };

    std::fs::create_dir_all(req.output_dir)
        .with_context(|| format!("create stem output dir {}", req.output_dir.display()))?;

    let ext = req.format.extension();
    let paths = [
        req.output_dir.join(format!("vocals.{ext}")),
        req.output_dir.join(format!("drums.{ext}")),
        req.output_dir.join(format!("bass.{ext}")),
        req.output_dir.join(format!("other.{ext}")),
    ];

    let stem_indices = resolve_stem_indices(&names);
    // ponytail: full-stem f32 buffers in RAM; streaming encode if cache size hurts.
    let mut buffers: [Vec<f32>; 4] = [
        Vec::with_capacity(n * 2),
        Vec::with_capacity(n * 2),
        Vec::with_capacity(n * 2),
        Vec::with_capacity(n * 2),
    ];

    let mut left_raw = vec![0f32; win];
    let mut right_raw = vec![0f32; win];
    let mut pos = 0usize;
    let windows_total = if hop == 0 { 0 } else { n.div_ceil(hop) };
    let mut windows_done = 0usize;

    while pos < n {
        fill_stereo_window(&samples, 2, pos, &mut left_raw, &mut right_raw);
        let out = engine::run_window_demucs(&left_raw, &right_raw).map_err(ssc_err)?;
        let (stems_count, _ch, t_out) = (out.shape()[0], out.shape()[1], out.shape()[2]);
        let copy_len = hop.min(t_out).min(n - pos);

        for (slot, &stem_idx) in stem_indices.iter().enumerate() {
            let idx = stem_idx.min(stems_count.saturating_sub(1));
            for i in 0..copy_len {
                buffers[slot].push(out[(idx, 0, i)]);
                buffers[slot].push(out[(idx, 1, i)]);
            }
        }

        windows_done += 1;
        if let Some(cb) = req.on_window_progress.as_ref() {
            cb(windows_done, windows_total.max(1));
        }

        if pos + hop >= n {
            break;
        }
        pos += hop;
    }

    let mut file_rate = mf.sample_rate;
    for (i, buf) in buffers.iter().enumerate() {
        let rate = encode_stem_file(&paths[i], req.format, buf, mf.sample_rate)
            .with_context(|| format!("encode {}", paths[i].display()))?;
        file_rate = rate;
    }

    Ok(StemSplitResult {
        paths,
        sample_rate: file_rate,
        backend: model_name.to_string(),
    })
}

fn resolve_stem_indices(names: &[String]) -> [usize; 4] {
    let mut name_idx: HashMap<String, usize> = HashMap::new();
    for (i, name) in names.iter().enumerate() {
        name_idx.insert(name.to_lowercase(), i);
    }
    let get_idx =
        |key: &str, fallback: usize| -> usize { name_idx.get(key).copied().unwrap_or(fallback) };
    [
        get_idx("vocals", 0),
        get_idx("drums", 1),
        get_idx("bass", 2),
        get_idx("other", 3),
    ]
}

fn ssc_err(err: impl std::fmt::Display) -> anyhow::Error {
    anyhow!("{err}")
}

fn resample_interleaved_linear(input: &[f32], from_hz: u32, to_hz: u32) -> Vec<f32> {
    if from_hz == 0 || to_hz == 0 || input.len() < 2 {
        return Vec::new();
    }
    let in_frames = input.len() / 2;
    let last = in_frames.saturating_sub(1);
    let out_frames = (in_frames as u64 * u64::from(to_hz) / u64::from(from_hz)) as usize;
    let ratio = f64::from(from_hz) / f64::from(to_hz);
    let mut out = Vec::with_capacity(out_frames.saturating_mul(2));
    for i in 0..out_frames {
        let src = i as f64 * ratio;
        let i0 = (src.floor() as usize).min(last);
        let frac = (src - i0 as f64) as f32;
        let i1 = (i0 + 1).min(last);
        for ch in 0..2 {
            let a = input[i0 * 2 + ch];
            let b = input[i1 * 2 + ch];
            out.push(a + (b - a) * frac);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest_from_json(json: &str) -> ModelManifest {
        serde_json::from_str(json).expect("manifest json")
    }

    #[test]
    fn linear_resample_48000_to_44100_changes_frame_count() {
        let frames = 4800usize;
        let input = vec![0.25f32; frames * 2];
        let out = resample_interleaved_linear(&input, 48_000, 44_100);
        assert_eq!(out.len() / 2, 4410);
    }

    #[test]
    fn ensure_reuses_existing_artifact_under_models_root() {
        let root = tempfile::tempdir().expect("tempdir");
        let content = b"mixar-test-model";
        // sha256("mixar-test-model")
        let sha = "4777239d151b012185acba78386c96c57c777c1998d5cda9ea917c1a19153cde";
        let manifest = manifest_from_json(&format!(
            r#"{{
              "name": "mixar_test",
              "sample_rate": 44100,
              "window": 1,
              "hop": 1,
              "artifacts": [{{
                "file": "m.onnx",
                "sha256": "{sha}",
                "size_bytes": {},
                "url": "http://127.0.0.1:9/unreachable.onnx"
              }}]
            }}"#,
            content.len()
        ));

        let expected = artifact_path(root.path(), &manifest, "m.onnx", sha).expect("path");
        std::fs::write(&expected, content).expect("write fixture");

        let handle = ensure_from_manifest(root.path(), &manifest).expect("reuse");
        assert_eq!(handle.local_path, expected);
        assert!(handle.local_path.starts_with(root.path()));
        assert_eq!(std::fs::read(&handle.local_path).unwrap(), content);
    }

    #[test]
    fn artifact_path_uses_models_root_only() {
        let root = Path::new("/tmp/mixar-models");
        let manifest = manifest_from_json(
            r#"{
              "name": "htdemucs_ort_v1",
              "sample_rate": 44100,
              "window": 1,
              "hop": 1
            }"#,
        );
        let path = artifact_path(root, &manifest, "model.onnx", "abcdef0123456789").unwrap();
        assert_eq!(
            path,
            PathBuf::from("/tmp/mixar-models/htdemucs_ort_v1-abcdef01.onnx")
        );
    }

    #[test]
    fn artifact_path_rejects_traversal_and_absolute_names() {
        let root = Path::new("/tmp/mixar-models");
        let traversal = manifest_from_json(
            r#"{
              "name": "../escape",
              "sample_rate": 44100,
              "window": 1,
              "hop": 1
            }"#,
        );
        assert!(artifact_path(root, &traversal, "m.onnx", "abcdef01").is_err());

        let absolute = manifest_from_json(
            r#"{
              "name": "/etc/passwd",
              "sample_rate": 44100,
              "window": 1,
              "hop": 1
            }"#,
        );
        assert!(artifact_path(root, &absolute, "m.onnx", "abcdef01").is_err());

        let safe = manifest_from_json(
            r#"{
              "name": "ok",
              "sample_rate": 44100,
              "window": 1,
              "hop": 1
            }"#,
        );
        assert!(artifact_path(root, &safe, "../../evil.onnx", "abcdef01").is_err());
    }

    #[test]
    fn ensure_from_manifest_rejects_traversal_before_download() {
        let root = tempfile::tempdir().expect("tempdir");
        let manifest = manifest_from_json(
            r#"{
              "name": "../escape",
              "sample_rate": 44100,
              "window": 1,
              "hop": 1,
              "artifacts": [{
                "file": "m.onnx",
                "sha256": "4777239d151b012185acba78386c96c57c777c1998d5cda9ea917c1a19153cde",
                "size_bytes": 1,
                "url": "http://127.0.0.1:9/unreachable.onnx"
              }]
            }"#,
        );
        assert!(ensure_from_manifest(root.path(), &manifest).is_err());
        assert_eq!(root.path().read_dir().unwrap().count(), 0);
    }
}
