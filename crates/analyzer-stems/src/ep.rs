//! ORT execution-provider preference cascade.
//!
//! Prefer WebGPU when the `webgpu` feature is on, always end on CPU. Optional
//! force via `MIXAR_STEMS_ORT_EP` (e.g. `cpu`, `webgpu`).

use ort::ep::{self, ExecutionProviderDispatch};

/// Env var that forces a single EP label (skips the platform cascade).
pub const FORCE_EP_ENV: &str = "MIXAR_STEMS_ORT_EP";

/// Short Mixar cache/backend label for an EP name (`cpu`, `webgpu`, …).
///
/// Accepts either our short labels or ORT `*ExecutionProvider` names.
/// Unknown names fall back to `"cpu"`.
pub fn ep_label(name: &str) -> &'static str {
    parse_ep_label(name).unwrap_or("cpu")
}

/// Value of [`FORCE_EP_ENV`] when set to a recognized label.
pub fn force_ep_from_env() -> Option<&'static str> {
    let raw = std::env::var(FORCE_EP_ENV).ok()?;
    parse_ep_label(&raw)
}

/// EP builders in preference order for the current platform / force env.
pub fn preferred_execution_providers() -> Vec<ExecutionProviderDispatch> {
    preferred_ep_cascade()
        .into_iter()
        .map(|(_, ep)| ep)
        .collect()
}

/// Short labels in the same order as [`preferred_execution_providers`].
pub fn preferred_ep_labels() -> Vec<&'static str> {
    preferred_ep_cascade()
        .into_iter()
        .map(|(label, _)| label)
        .collect()
}

/// `(label, EP)` pairs in preference order (for sequential session tries).
pub(crate) fn preferred_ep_cascade() -> Vec<(&'static str, ExecutionProviderDispatch)> {
    preference_cascade()
}

fn normalize_ep_key(name: &str) -> String {
    name.trim().to_ascii_lowercase()
}

fn parse_ep_label(name: &str) -> Option<&'static str> {
    match normalize_ep_key(name).as_str() {
        "cpu" | "cpuexecutionprovider" => Some("cpu"),
        "webgpu" | "webgpuexecutionprovider" => Some("webgpu"),
        // Recognized for cache/env docs; not compiled in (no joint ORT EP binary
        // with webgpu — enabling these as cargo features breaks `--all-features`).
        "cuda" | "cudaexecutionprovider" => Some("cuda"),
        "tensorrt" | "tensorrtexecutionprovider" => Some("tensorrt"),
        "tensorrt_rtx" | "nvtensorrtrtxexecutionprovider" | "nvrtx" => Some("tensorrt_rtx"),
        "coreml" | "coremlexecutionprovider" => Some("coreml"),
        "directml" | "dmlexecutionprovider" | "dml" => Some("directml"),
        "openvino" | "openvinoexecutionprovider" => Some("openvino"),
        _ => None,
    }
}

fn preference_cascade() -> Vec<(&'static str, ExecutionProviderDispatch)> {
    if let Some(label) = force_ep_from_env() {
        return vec![ep_entry(label).unwrap_or_else(cpu_entry)];
    }

    let mut out = Vec::new();

    #[cfg(any(target_os = "linux", target_os = "windows", target_vendor = "apple"))]
    {
        #[cfg(feature = "webgpu")]
        out.push(ep_entry("webgpu").expect("webgpu feature"));
    }

    out.push(cpu_entry());
    out
}

fn cpu_entry() -> (&'static str, ExecutionProviderDispatch) {
    // CPU may remain the only EP; silent failure is fine.
    ("cpu", ep::CPU::default().build())
}

fn ep_entry(label: &'static str) -> Option<(&'static str, ExecutionProviderDispatch)> {
    // Non-CPU EPs must `.error_on_failure()`: ort's default is fail-silently and
    // still commit a CPU session, which would make our cascade think the GPU EP
    // won while inference stays on CPU.
    let ep = match label {
        "cpu" => ep::CPU::default().build(),
        #[cfg(feature = "webgpu")]
        "webgpu" => webgpu_ep().error_on_failure(),
        _ => return None,
    };
    Some((label, ep))
}

#[cfg(feature = "webgpu")]
fn webgpu_ep() -> ExecutionProviderDispatch {
    // Bare EP registration. ort 2.0.0-rc.11's with_* helpers pass keys already
    // prefixed `ep.webgpuexecutionprovider.*` into AppendExecutionProvider,
    // which prefixes again — options never apply. Real options are set on the
    // SessionBuilder in `session::try_commit` via with_config_entry.
    force_c_numeric_locale();
    ep::WebGPU::default().build()
}

/// ORT WebGPU embeds floats into WGSL with locale-sensitive C printf.
/// Flutter calls `setlocale(LC_ALL, "")` at startup; under pt_BR that yields
/// `f32(0,000010)` which Dawn rejects as "integer literal cannot have leading 0s".
#[cfg(feature = "webgpu")]
pub(crate) fn force_c_numeric_locale() {
    #[cfg(unix)]
    {
        let prev = unsafe { libc::setlocale(libc::LC_NUMERIC, c"C".as_ptr()) };
        if !prev.is_null() {
            let previous = unsafe { std::ffi::CStr::from_ptr(prev) }.to_string_lossy();
            if previous != "C" && previous != "POSIX" {
                tracing::info!(%previous, "stem ort: LC_NUMERIC=C for WebGPU WGSL floats");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn with_force_ep<R>(value: Option<&str>, f: impl FnOnce() -> R) -> R {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        match value {
            Some(v) => std::env::set_var(FORCE_EP_ENV, v),
            None => std::env::remove_var(FORCE_EP_ENV),
        }
        let out = f();
        std::env::remove_var(FORCE_EP_ENV);
        out
    }

    #[test]
    fn force_cpu_returns_cpu_only_list() {
        with_force_ep(Some("cpu"), || {
            assert_eq!(force_ep_from_env(), Some("cpu"));
            assert_eq!(preferred_ep_labels(), vec!["cpu"]);
            assert_eq!(preferred_execution_providers().len(), 1);
        });
    }

    #[test]
    fn ep_label_maps_ort_and_short_names() {
        assert_eq!(ep_label("CPUExecutionProvider"), "cpu");
        assert_eq!(ep_label("cuda"), "cuda");
        assert_eq!(ep_label("WebGpuExecutionProvider"), "webgpu");
        assert_eq!(ep_label("DmlExecutionProvider"), "directml");
        assert_eq!(ep_label("NvTensorRTRTXExecutionProvider"), "tensorrt_rtx");
    }

    #[test]
    fn default_cascade_ends_with_cpu() {
        with_force_ep(None, || {
            let labels = preferred_ep_labels();
            assert_eq!(labels.last().copied(), Some("cpu"));
            assert!(!labels.is_empty());
        });
    }

    #[cfg(all(feature = "webgpu", target_os = "linux"))]
    #[test]
    fn linux_webgpu_feature_orders_webgpu_before_cpu() {
        with_force_ep(None, || {
            let labels = preferred_ep_labels();
            let webgpu = labels.iter().position(|&l| l == "webgpu");
            let cpu = labels.iter().position(|&l| l == "cpu");
            assert_eq!(
                webgpu,
                Some(0),
                "expected webgpu first when only webgpu+cpu: {labels:?}"
            );
            assert_eq!(cpu, Some(1), "expected cpu after webgpu: {labels:?}");
        });
    }

    #[cfg(all(feature = "webgpu", unix))]
    #[test]
    fn force_c_numeric_locale_makes_printf_use_dot() {
        unsafe {
            let pt = std::ffi::CString::new("pt_BR.UTF-8").unwrap();
            libc::setlocale(libc::LC_NUMERIC, pt.as_ptr());
        }
        force_c_numeric_locale();
        let mut buf = [0i8; 64];
        unsafe {
            libc::snprintf(buf.as_mut_ptr(), buf.len(), c"%f".as_ptr(), 0.00001f64);
        }
        let s = unsafe { std::ffi::CStr::from_ptr(buf.as_ptr()) }
            .to_string_lossy()
            .into_owned();
        assert!(
            s.contains('.') && !s.contains(','),
            "expected C-locale float, got {s:?}"
        );
    }
}
