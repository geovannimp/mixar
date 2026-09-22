//! Prefer a working ORT execution provider for Demucs (skip broken CUDA).

use std::path::{Path, PathBuf};
use std::sync::Once;

/// Call once before [`stem_splitter_core::core::engine::preload`].
///
/// When CUDA provider / toolkit / GPU device are missing, force stem-splitter to
/// skip CUDA so OneDNN / XNNPACK can win instead of a silent CPU session that
/// claimed to be CUDA.
pub fn prepare_ort_execution_providers() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        if std::env::var_os("STEMMER_EP_FORCE").is_some()
            || std::env::var_os("STEMMER_FORCE_CPU").is_some()
        {
            return;
        }
        match cuda_readiness() {
            CudaReadiness::Ready => {
                if !preload_ort_cuda_provider_libs() {
                    disable_cuda_ep("CUDA provider preload failed");
                }
            }
            CudaReadiness::Unusable(reason) => disable_cuda_ep(&reason),
        }
    });
}

enum CudaReadiness {
    Ready,
    Unusable(String),
}

fn cuda_readiness() -> CudaReadiness {
    #[cfg(not(target_os = "linux"))]
    {
        CudaReadiness::Unusable("CUDA EP is Linux/Windows only in this build".into())
    }
    #[cfg(target_os = "linux")]
    {
        if !Path::new("/dev/nvidia0").exists() && !Path::new("/dev/nvidiactl").exists() {
            return CudaReadiness::Unusable(
                "no NVIDIA device node (/dev/nvidia0); is the driver loaded?".into(),
            );
        }
        if find_ort_provider_lib("libonnxruntime_providers_cuda.so").is_none() {
            return CudaReadiness::Unusable(
                "libonnxruntime_providers_cuda.so not found under ~/.cache/ort.pyke.io".into(),
            );
        }
        let missing = missing_cuda_runtime_libs();
        if !missing.is_empty() {
            return CudaReadiness::Unusable(format!(
                "GPU present but CUDA 12 runtime libs missing ({}); \
install e.g. `sudo apt install libcudart12 libcublas12 libcublaslt12 libcufft11 libcurand10 nvidia-cudnn`",
                missing.join(", ")
            ));
        }
        CudaReadiness::Ready
    }
}

/// Shared libs ORT's CUDA EP dlopens (see `ldd libonnxruntime_providers_cuda.so`).
fn missing_cuda_runtime_libs() -> Vec<&'static str> {
    const NEEDED: &[&str] = &[
        "libcudart.so.12",
        "libcublas.so.12",
        "libcublasLt.so.12",
        "libcufft.so.11",
        "libcurand.so.10",
        "libcudnn.so.9",
    ];
    NEEDED
        .iter()
        .copied()
        // SAFETY: only probing whether the shared library exists/loads.
        .filter(|name| unsafe { libloading::Library::new(name).is_err() })
        .collect()
}

fn disable_cuda_ep(reason: &str) {
    const KEY: &str = "STEMMER_EP_DISABLE";
    let existing = std::env::var(KEY).unwrap_or_default();
    let lower = existing.to_ascii_lowercase();
    if !lower.split(',').any(|t| t.trim() == "cuda") {
        let next = if existing.trim().is_empty() {
            "cuda".to_string()
        } else {
            format!("{existing},cuda")
        };
        // SAFETY: single-threaded Once before stem sessions start.
        unsafe { std::env::set_var(KEY, &next) };
        eprintln!("mixar stems: skipping CUDA EP ({reason}); STEMMER_EP_DISABLE={next}");
    }
    // ORT can create a CPU session after a failed CUDA register and
    // stem-splitter may cache that as a healthy CUDA probe — scrub it.
    scrub_cuda_probe_success_cache();
}

fn find_ort_provider_lib(file_name: &str) -> Option<PathBuf> {
    if let Ok(dir) = std::env::var("ORT_CACHE_DIR") {
        if let Some(p) = find_named_under(Path::new(&dir), file_name) {
            return Some(p);
        }
    }
    if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
        let cache = home.join(".cache/ort.pyke.io");
        if let Some(p) = find_named_under(&cache, file_name) {
            return Some(p);
        }
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let direct = dir.join(file_name);
            if direct.is_file() {
                return Some(direct);
            }
            let lib = dir.join("lib").join(file_name);
            if lib.is_file() {
                return Some(lib);
            }
        }
    }
    None
}

fn find_named_under(root: &Path, file_name: &str) -> Option<PathBuf> {
    if !root.is_dir() {
        return None;
    }
    // Bound walk: ort cache is shallow (dfbin/<triple>/<hash>/).
    let mut stack = vec![(root.to_path_buf(), 0u8)];
    while let Some((dir, depth)) = stack.pop() {
        if depth > 6 {
            continue;
        }
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push((path, depth + 1));
            } else if path.file_name().and_then(|s| s.to_str()) == Some(file_name) {
                return Some(path);
            }
        }
    }
    None
}

/// Drop false-positive CUDA entries from stem-splitter-core's probe cache
/// (`~/.cache/stem-splitter-core/ep_probe_success_v1.json` on Linux).
fn scrub_cuda_probe_success_cache() {
    let Some(home) = std::env::var_os("HOME").map(PathBuf::from) else {
        return;
    };
    let path = home
        .join(".cache")
        .join("stem-splitter-core")
        .join("ep_probe_success_v1.json");
    let Ok(raw) = std::fs::read_to_string(&path) else {
        return;
    };
    let Ok(mut value) = serde_json::from_str::<serde_json::Value>(&raw) else {
        return;
    };
    let removed = {
        let Some(entries) = value.get_mut("entries").and_then(|e| e.as_array_mut()) else {
            return;
        };
        let before = entries.len();
        entries.retain(|entry| {
            entry
                .get("provider")
                .and_then(|p| p.as_str())
                .map(|p| p != "cuda")
                .unwrap_or(true)
        });
        before - entries.len()
    };
    if removed == 0 {
        return;
    }
    if let Ok(out) = serde_json::to_string_pretty(&value) {
        let _ = std::fs::write(&path, out);
        eprintln!(
            "mixar stems: cleared {removed} stale CUDA probe-success entr(y/ies) in {}",
            path.display()
        );
    }
}

fn preload_ort_cuda_provider_libs() -> bool {
    for name in [
        "libonnxruntime_providers_shared.so",
        "libonnxruntime_providers_cuda.so",
    ] {
        let Some(path) = find_ort_provider_lib(name) else {
            eprintln!("mixar stems: missing ORT provider lib {name}");
            return false;
        };
        match unsafe { libloading::Library::new(&path) } {
            Ok(lib) => {
                std::mem::forget(lib);
                eprintln!("mixar stems: preloaded {}", path.display());
            }
            Err(err) => {
                eprintln!("mixar stems: failed to preload {}: {err}", path.display());
                return false;
            }
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_named_under_misses_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(find_named_under(dir.path(), "nope.so").is_none());
    }
}
