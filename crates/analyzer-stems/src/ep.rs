//! ORT execution-provider preference cascade.
//!
//! Prefer the fastest available EP, always end on CPU. Optional force via
//! `MIXAR_STEMS_ORT_EP` (e.g. `cpu`, `cuda`, `webgpu`). Cargo features gate
//! which non-CPU EPs are offered; CI stays on default (CPU only).

use ort::ep::{self, ExecutionProviderDispatch};

/// Env var that forces a single EP label (skips the platform cascade).
pub const FORCE_EP_ENV: &str = "MIXAR_STEMS_ORT_EP";

/// Short Mixar cache/backend label for an EP name (`cpu`, `cuda`, …).
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
    preference_cascade().into_iter().map(|(_, ep)| ep).collect()
}

/// Short labels in the same order as [`preferred_execution_providers`].
pub fn preferred_ep_labels() -> Vec<&'static str> {
    preference_cascade()
        .into_iter()
        .map(|(label, _)| label)
        .collect()
}

fn normalize_ep_key(name: &str) -> String {
    name.trim().to_ascii_lowercase()
}

fn parse_ep_label(name: &str) -> Option<&'static str> {
    match normalize_ep_key(name).as_str() {
        "cpu" | "cpuexecutionprovider" => Some("cpu"),
        "cuda" | "cudaexecutionprovider" => Some("cuda"),
        "tensorrt" | "tensorrtexecutionprovider" => Some("tensorrt"),
        "tensorrt_rtx" | "nvtensorrtrtxexecutionprovider" | "nvrtx" => Some("tensorrt_rtx"),
        "webgpu" | "webgpuexecutionprovider" => Some("webgpu"),
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

    #[cfg(any(target_os = "linux", target_os = "windows"))]
    {
        #[cfg(feature = "nvrtx")]
        out.push(ep_entry("tensorrt_rtx").expect("nvrtx feature"));
        #[cfg(feature = "tensorrt")]
        out.push(ep_entry("tensorrt").expect("tensorrt feature"));
        #[cfg(feature = "cuda")]
        out.push(ep_entry("cuda").expect("cuda feature"));
    }

    #[cfg(target_os = "windows")]
    {
        #[cfg(feature = "directml")]
        out.push(ep_entry("directml").expect("directml feature"));
    }

    #[cfg(target_vendor = "apple")]
    {
        #[cfg(feature = "coreml")]
        out.push(ep_entry("coreml").expect("coreml feature"));
    }

    #[cfg(any(target_os = "linux", target_os = "windows", target_vendor = "apple"))]
    {
        #[cfg(feature = "webgpu")]
        out.push(ep_entry("webgpu").expect("webgpu feature"));
    }

    #[cfg(all(feature = "openvino", target_os = "linux"))]
    out.push(ep_entry("openvino").expect("openvino feature"));

    out.push(cpu_entry());
    out
}

fn cpu_entry() -> (&'static str, ExecutionProviderDispatch) {
    ("cpu", ep::CPU::default().build())
}

fn ep_entry(label: &'static str) -> Option<(&'static str, ExecutionProviderDispatch)> {
    let ep = match label {
        "cpu" => ep::CPU::default().build(),
        #[cfg(feature = "cuda")]
        "cuda" => ep::CUDA::default().build(),
        #[cfg(feature = "tensorrt")]
        "tensorrt" => ep::TensorRT::default().build(),
        #[cfg(feature = "nvrtx")]
        "tensorrt_rtx" => ep::NVRTX::default().build(),
        #[cfg(feature = "webgpu")]
        "webgpu" => ep::WebGPU::default().build(),
        #[cfg(feature = "coreml")]
        "coreml" => ep::CoreML::default().build(),
        #[cfg(feature = "directml")]
        "directml" => ep::DirectML::default().build(),
        #[cfg(feature = "openvino")]
        "openvino" => ep::OpenVINO::default().build(),
        _ => return None,
    };
    Some((label, ep))
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
}
