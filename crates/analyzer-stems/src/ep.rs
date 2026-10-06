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
    // Bare EP registration — with_* helpers pre-prefix `ep.webgpuexecutionprovider.*`,
    // which AppendExecutionProvider prefixes again, so options never apply. Use defaults.
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
        /// Restores the previous env value even if `f` panics.
        struct Restore(Option<std::ffi::OsString>);
        impl Drop for Restore {
            fn drop(&mut self) {
                match self.0.take() {
                    Some(v) => std::env::set_var(FORCE_EP_ENV, v),
                    None => std::env::remove_var(FORCE_EP_ENV),
                }
            }
        }

        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        // Declared after `_guard`, so it drops first (reverse declaration order):
        // the restore happens while the lock is still held.
        let _restore = Restore(std::env::var_os(FORCE_EP_ENV));
        match value {
            Some(v) => std::env::set_var(FORCE_EP_ENV, v),
            None => std::env::remove_var(FORCE_EP_ENV),
        }
        f()
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

    /// Restores the process-global `LC_NUMERIC` on drop, including on unwind.
    #[cfg(all(feature = "webgpu", unix))]
    struct LocaleGuard(std::ffi::CString);

    #[cfg(all(feature = "webgpu", unix))]
    impl LocaleGuard {
        /// Capture the current locale. Call while holding `ENV_LOCK`.
        fn capture() -> Self {
            // SAFETY: `setlocale(LC_NUMERIC, NULL)` is the query form — it does
            // not modify process state. The returned pointer is owned by libc and
            // valid only until the next `setlocale`, so it is copied immediately
            // into a `CString` before any further `setlocale` call.
            let current = unsafe { libc::setlocale(libc::LC_NUMERIC, std::ptr::null()) };
            assert!(!current.is_null(), "LC_NUMERIC query failed");
            Self(unsafe { std::ffi::CStr::from_ptr(current) }.to_owned())
        }
    }

    #[cfg(all(feature = "webgpu", unix))]
    impl Drop for LocaleGuard {
        fn drop(&mut self) {
            // SAFETY: `self.0` is an owned `CString` that outlives the call, so
            // the `*const c_char` passed to `setlocale` is valid for its duration.
            // This is process-global mutation, serialized by `ENV_LOCK`.
            unsafe {
                libc::setlocale(libc::LC_NUMERIC, self.0.as_ptr());
            }
        }
    }

    /// Format `value` with libc `%f` — the same call site the WebGPU/WGSL float
    /// path depends on.
    #[cfg(all(feature = "webgpu", unix))]
    fn format_float(value: f64) -> String {
        let mut buf = [0 as libc::c_char; 64];
        // SAFETY: `buf` is a valid writable array of `buf.len()` bytes and the
        // format string is a static NUL-terminated literal taking one `f64`.
        let written = unsafe { libc::snprintf(buf.as_mut_ptr(), buf.len(), c"%f".as_ptr(), value) };
        // Fail loudly rather than assert on a silently truncated rendering.
        assert!(
            written >= 0 && (written as usize) < buf.len(),
            "snprintf truncated {value} (wrote {written}, buffer {})",
            buf.len()
        );
        // SAFETY: `snprintf` NUL-terminates whenever the buffer size is non-zero
        // (asserted above), and `buf` outlives the call.
        unsafe { std::ffi::CStr::from_ptr(buf.as_ptr()) }
            .to_string_lossy()
            .into_owned()
    }

    /// Candidate comma-decimal locales, most likely first.
    #[cfg(all(feature = "webgpu", unix))]
    const COMMA_LOCALES: &[&str] = &[
        "pt_BR.UTF-8",
        "pt_BR.utf8",
        "de_DE.UTF-8",
        "de_DE.utf8",
        "fr_FR.UTF-8",
        "fr_FR.utf8",
        "es_ES.UTF-8",
        "es_ES.utf8",
        "it_IT.UTF-8",
        "it_IT.utf8",
    ];

    /// Activate the first installed comma-decimal locale and return its name, or
    /// `None` when the host has none (e.g. `ubuntu-latest`). Also reports how many
    /// candidates were installed but turned out to use a dot.
    #[cfg(all(feature = "webgpu", unix))]
    fn install_first_comma_locale() -> (Option<&'static str>, usize) {
        let mut installed_but_dot = 0usize;
        for name in COMMA_LOCALES {
            let Ok(c) = std::ffi::CString::new(*name) else {
                continue;
            };
            // SAFETY: `c` is an owned `CString` that outlives the call. Mutates
            // process-global `LC_NUMERIC`, serialized by `ENV_LOCK`.
            let set = unsafe { libc::setlocale(libc::LC_NUMERIC, c.as_ptr()) };
            if set.is_null() {
                continue; // not installed on this host
            }
            if format_float(0.5).contains(',') {
                return (Some(name), installed_but_dot);
            }
            installed_but_dot += 1;
        }
        (None, installed_but_dot)
    }

    /// Production requirement: after `force_c_numeric_locale`, printf always uses
    /// a dot. This holds regardless of the host's locales, so it is safe on CI;
    /// where a comma-decimal locale exists it is established first, so the default
    /// run really exercises the override rather than trivially satisfying the
    /// assertion on a dot-emitting host.
    #[cfg(all(feature = "webgpu", unix))]
    #[test]
    fn force_c_numeric_locale_keeps_printf_dot() {
        // LC_NUMERIC is process-global C state, so serialize with the other
        // env-mutating tests. The guard restores it on drop, including on unwind.
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let _restore_locale = LocaleGuard::capture();

        let (comma_locale, _) = install_first_comma_locale();
        let before = format_float(0.5);
        if let Some(name) = comma_locale {
            assert!(
                before.contains(','),
                "expected {name} to format with a comma, got {before:?}"
            );
        }

        force_c_numeric_locale();
        let s = format_float(0.00001);
        assert!(
            s.contains('.') && !s.contains(','),
            "force_c_numeric_locale must format with a dot, got {s:?} \
             (precondition comma locale: {comma_locale:?}, before: {before:?})"
        );
    }

    /// Stronger check: with a comma-decimal locale *active*, forcing the C locale
    /// must still switch printf to a dot.
    ///
    /// Ignored by default because GitHub's `ubuntu-latest` image ships no
    /// comma-decimal locale, so the precondition cannot be created there. Run it
    /// explicitly with `cargo test -p analyzer-stems -- --ignored`.
    #[cfg(all(feature = "webgpu", unix))]
    #[test]
    #[ignore = "needs a comma-decimal locale (e.g. pt_BR.UTF-8); not installed on ubuntu-latest"]
    fn force_c_numeric_locale_overrides_comma_locale() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let _restore_locale = LocaleGuard::capture();

        let (available, installed_but_dot) = install_first_comma_locale();
        let forced = available.map(|_| {
            force_c_numeric_locale();
            format_float(0.00001)
        });

        let s = forced.unwrap_or_else(|| {
            panic!(
                "no comma-decimal locale available (tried {COMMA_LOCALES:?}); \
                 {installed_but_dot} are installed but format with a dot"
            )
        });
        assert!(
            s.contains('.') && !s.contains(','),
            "expected C-locale float, got {s:?}"
        );
    }
}
