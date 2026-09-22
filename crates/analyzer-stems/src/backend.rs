//! Which Burn backend runs the HTDemucs forward pass.

use std::sync::OnceLock;

/// Compute backend chosen for a split.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComputeBackend {
    /// CPU (`burn-ndarray`), always available.
    NdArray,
    /// GPU (`burn-wgpu`).
    Wgpu,
}

impl ComputeBackend {
    /// Suffix reported in [`crate::StemSplitResult::backend`] after the model id.
    pub fn label(self) -> &'static str {
        match self {
            Self::NdArray => "ndarray",
            Self::Wgpu => "wgpu",
        }
    }
}

/// Pick the backend to separate with.
///
/// Order: `MIXAR_STEMS_FORCE_CPU=1` → ndarray; else try wgpu (when featured) and
/// fall back to ndarray if no adapter is present.
pub fn select_backend() -> ComputeBackend {
    choose_backend(env_flag("MIXAR_STEMS_FORCE_CPU"), wgpu_usable())
}

fn choose_backend(force_cpu: bool, wgpu_ok: bool) -> ComputeBackend {
    if force_cpu {
        return ComputeBackend::NdArray;
    }
    if wgpu_ok {
        ComputeBackend::Wgpu
    } else {
        ComputeBackend::NdArray
    }
}

fn env_flag(name: &str) -> bool {
    matches!(std::env::var(name).as_deref(), Ok("1"))
}

fn wgpu_usable() -> bool {
    #[cfg(feature = "burn-wgpu")]
    {
        wgpu_adapter_available()
    }
    #[cfg(not(feature = "burn-wgpu"))]
    {
        false
    }
}

/// True when a high-performance wgpu adapter exists (Vulkan/Metal/DX12/…).
///
/// Probes with the `wgpu` crate rather than Burn init: Mixar release builds use
/// `panic = "abort"`, so a failing Burn `.expect` would take down the process
/// instead of falling back to ndarray.
#[cfg(feature = "burn-wgpu")]
fn wgpu_adapter_available() -> bool {
    static AVAILABLE: OnceLock<bool> = OnceLock::new();
    *AVAILABLE.get_or_init(|| {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback_adapter: false,
            compatible_surface: None,
        }));
        match adapter {
            Ok(adapter) => {
                tracing::info!(info = ?adapter.get_info(), "stem wgpu adapter available");
                true
            }
            Err(err) => {
                tracing::warn!(
                    ?err,
                    "no wgpu adapter; stem backend falling back to ndarray"
                );
                false
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn force_cpu_selects_ndarray() {
        assert_eq!(choose_backend(true, true), ComputeBackend::NdArray);
        assert_eq!(choose_backend(true, false), ComputeBackend::NdArray);
    }

    #[test]
    fn wgpu_wins_when_available_and_not_forced() {
        assert_eq!(choose_backend(false, true), ComputeBackend::Wgpu);
        assert_eq!(choose_backend(false, false), ComputeBackend::NdArray);
    }

    #[test]
    fn labels_match_cache_suffix() {
        assert_eq!(ComputeBackend::NdArray.label(), "ndarray");
        assert_eq!(ComputeBackend::Wgpu.label(), "wgpu");
    }
}
