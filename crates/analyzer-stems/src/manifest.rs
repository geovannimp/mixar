#![allow(dead_code)] // pub API wired in Task 3 (ORT infer)
//! Mixar-owned model manifest, download, and path safety.

use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Default StemSplit HTDemucs ONNX model id (ORT path).
pub const DEFAULT_MODEL: &str = "htdemucs_ort_v2";

/// On-disk artifact descriptor in a Mixar model manifest.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Artifact {
    pub file: String,
    pub sha256: String,
    pub size_bytes: u64,
    pub url: String,
    #[serde(default)]
    pub format: String,
}

/// Mixar model manifest (JSON-serializable).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ModelManifest {
    pub name: String,
    pub sample_rate: u32,
    #[serde(default)]
    pub artifacts: Vec<Artifact>,
}

/// Resolved model with local artifact path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelHandle {
    pub manifest: ModelManifest,
    pub local_path: PathBuf,
}

impl ModelManifest {
    /// Primary weight artifact (first entry).
    pub fn resolve_primary_artifact(&self) -> Result<&Artifact> {
        self.artifacts
            .first()
            .ok_or_else(|| anyhow!("model manifest has no artifacts"))
    }
}

/// Built-in manifest for known model ids (no network fetch).
pub fn builtin_manifest(model_name: &str) -> Result<ModelManifest> {
    match model_name {
        DEFAULT_MODEL => Ok(ModelManifest {
            name: DEFAULT_MODEL.into(),
            sample_rate: 44_100,
            artifacts: vec![Artifact {
                file: "htdemucs_fp16weights.onnx".into(),
                // sha256 pinned from Hugging Face StemSplitio/htdemucs-onnx LFS oid
                sha256: "d05c269d0178d2a72ad484b10b11dd370193fc923201c3b27a99f848745db70a".into(),
                size_bytes: 165_612_636,
                url: "https://huggingface.co/StemSplitio/htdemucs-onnx/resolve/main/htdemucs_fp16weights.onnx"
                    .into(),
                format: "onnx".into(),
            }],
        }),
        other => Err(anyhow!("unknown built-in stem model '{other}'")),
    }
}

/// Download / verify model weights into `models_root`.
pub fn ensure_model(models_root: &Path, model_name: &str) -> Result<()> {
    let handle = resolve_model(models_root, model_name, None)?;
    if !handle.local_path.is_file() {
        return Err(anyhow!(
            "model artifact missing at {}",
            handle.local_path.display()
        ));
    }
    Ok(())
}

/// Resolve (download if needed) a model into `models_root`.
pub fn resolve_model(
    models_root: &Path,
    model_name: &str,
    manifest_url_override: Option<&str>,
) -> Result<ModelHandle> {
    let manifest = match manifest_url_override {
        Some(url) => fetch_manifest(url)?,
        None => builtin_manifest(model_name)?,
    };
    ensure_from_manifest(models_root, &manifest)
}

/// Place / reuse the primary artifact under `models_root` from an already-fetched manifest.
pub fn ensure_from_manifest(models_root: &Path, manifest: &ModelManifest) -> Result<ModelHandle> {
    let artifact = manifest.resolve_primary_artifact()?;

    std::fs::create_dir_all(models_root)
        .with_context(|| format!("create models root {}", models_root.display()))?;

    let local_path = artifact_path(models_root, manifest, &artifact.file, &artifact.sha256)?;

    let need_download = !matches!(verify_sha256(&local_path, &artifact.sha256), Ok(true));
    if need_download {
        let client = http_client()?;
        download_file(&client, &artifact.url, &local_path)?;
        if !verify_sha256(&local_path, &artifact.sha256)? {
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

fn fetch_manifest(url: &str) -> Result<ModelManifest> {
    let client = http_client()?;
    client
        .get(url)
        .send()
        .context("fetch model manifest")?
        .error_for_status()
        .context("model manifest HTTP status")?
        .json()
        .context("parse model manifest JSON")
}

fn http_client() -> Result<Client> {
    Client::builder()
        .user_agent("mixar-analyzer-stems/1.0")
        .build()
        .context("create HTTP client")
}

fn download_file(client: &Client, url: &str, dest: &Path) -> Result<()> {
    let bytes = client
        .get(url)
        .send()
        .with_context(|| format!("download {url}"))?
        .error_for_status()
        .with_context(|| format!("download HTTP status for {url}"))?
        .bytes()
        .with_context(|| format!("read download body for {url}"))?;
    std::fs::write(dest, &bytes).with_context(|| format!("write {}", dest.display()))?;
    Ok(())
}

fn verify_sha256(path: &Path, expected: &str) -> Result<bool> {
    if !path.is_file() {
        return Ok(false);
    }
    let bytes = std::fs::read(path).with_context(|| format!("read {}", path.display()))?;
    let hash = hex::encode(Sha256::digest(bytes));
    Ok(hash.eq_ignore_ascii_case(expected))
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

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest_from_json(json: &str) -> ModelManifest {
        serde_json::from_str(json).expect("manifest json")
    }

    #[test]
    fn artifact_path_rejects_traversal() {
        let root = Path::new("/tmp/mixar-models");
        let m = ModelManifest {
            name: "../escape".into(),
            sample_rate: 44_100,
            artifacts: vec![],
        };
        assert!(artifact_path(root, &m, "m.onnx", "abcdef01").is_err());
    }

    #[test]
    fn builtin_manifest_htdemucs_ort_v2() {
        let m = builtin_manifest(DEFAULT_MODEL).expect("builtin");
        assert_eq!(m.name, DEFAULT_MODEL);
        assert_eq!(m.sample_rate, 44_100);
        let art = m.resolve_primary_artifact().expect("artifact");
        assert_eq!(art.file, "htdemucs_fp16weights.onnx");
        assert_eq!(art.format, "onnx");
        assert_eq!(art.size_bytes, 165_612_636);
        assert_eq!(
            art.sha256,
            "d05c269d0178d2a72ad484b10b11dd370193fc923201c3b27a99f848745db70a"
        );
        assert!(art.url.contains("htdemucs_fp16weights.onnx"));
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
              "name": "htdemucs_ort_v2",
              "sample_rate": 44100
            }"#,
        );
        let path = artifact_path(
            root,
            &manifest,
            "htdemucs_fp16weights.onnx",
            "d05c269d0178d2a72ad484b10b11dd370193fc923201c3b27a99f848745db70a",
        )
        .unwrap();
        assert_eq!(
            path,
            PathBuf::from("/tmp/mixar-models/htdemucs_ort_v2-d05c269d.onnx")
        );
    }

    #[test]
    fn artifact_path_rejects_traversal_and_absolute_names() {
        let root = Path::new("/tmp/mixar-models");
        let traversal = manifest_from_json(
            r#"{
              "name": "../escape",
              "sample_rate": 44100
            }"#,
        );
        assert!(artifact_path(root, &traversal, "m.onnx", "abcdef01").is_err());

        let absolute = manifest_from_json(
            r#"{
              "name": "/etc/passwd",
              "sample_rate": 44100
            }"#,
        );
        assert!(artifact_path(root, &absolute, "m.onnx", "abcdef01").is_err());

        let safe = manifest_from_json(
            r#"{
              "name": "ok",
              "sample_rate": 44100
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
