//! ORT execution-provider preference cascade.
//!
//! Desktop offline separation prefers WebGPU and always ends on CPU. Android
//! runs every stem job on CPU: NNAPI, XNNPACK, QNN and TVM all measured slower
//! for these models (see `PERF-RUN.md`). Optional force via `MIXAR_STEMS_ORT_EP`
//! (desktop) or the `debug.mixar.stems_ep` system property (Android, where an
//! app process cannot receive env vars) for experiments.

use ort::ep::{self, ExecutionProviderDispatch};

/// Env var that forces a single EP label (skips the platform cascade).
pub const FORCE_EP_ENV: &str = "MIXAR_STEMS_ORT_EP";

/// Android system property that mirrors [`FORCE_EP_ENV`].
#[cfg(target_os = "android")]
pub const FORCE_EP_PROPERTY: &str = "debug.mixar.stems_ep";

/// Short label for the CPU fallback.
pub const CPU_EP_LABEL: &str = "cpu";

/// Short label for the WebGPU EP.
pub const WEBGPU_EP_LABEL: &str = "webgpu";

/// Short label for the Android NNAPI EP.
pub const NNAPI_EP_LABEL: &str = "nnapi";

/// Short label for the XNNPACK EP, shared by the cascade and the live
/// single-thread policy.
pub const XNNPACK_EP_LABEL: &str = "xnnpack";

/// Short label for the Android QNN EP.
pub const QNN_EP_LABEL: &str = "qnn";

/// Probe-only labels for EPs ORT may expose but the cascade cannot force.
pub const ACL_EP_LABEL: &str = "acl";
pub const TVM_EP_LABEL: &str = "tvm";
pub const AZURE_EP_LABEL: &str = "azure";

/// Short Mixar cache/backend label for an EP name (`cpu`, `webgpu`, …).
///
/// Accepts either our short labels or ORT `*ExecutionProvider` names.
/// Unknown names fall back to `"cpu"`.
pub fn ep_label(name: &str) -> &'static str {
    parse_ep_label(name).unwrap_or(CPU_EP_LABEL)
}

/// Forced EP label from [`FORCE_EP_ENV`], or the Android system property.
pub fn force_ep_from_env() -> Option<&'static str> {
    let forced = std::env::var(FORCE_EP_ENV)
        .ok()
        .and_then(|raw| parse_ep_label(&raw));
    #[cfg(target_os = "android")]
    let forced = forced.or_else(android_forced_label);
    forced
}

/// `adb shell setprop debug.mixar.stems_ep nnapi` — the Android stand-in for an
/// env var. Read on each session build, so no app restart is needed.
#[cfg(target_os = "android")]
fn android_forced_label() -> Option<&'static str> {
    // `PROP_VALUE_MAX`, including the NUL terminator.
    let mut value = [0 as libc::c_char; 92];
    let name = std::ffi::CString::new(FORCE_EP_PROPERTY).ok()?;
    let len = unsafe { libc::__system_property_get(name.as_ptr(), value.as_mut_ptr()) };
    if len <= 0 {
        return None;
    }
    let raw = unsafe { std::ffi::CStr::from_ptr(value.as_ptr()) }
        .to_str()
        .ok()?;
    parse_ep_label(raw)
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
    labels_of(preferred_ep_cascade())
}

/// Short labels the realtime producer will try, in order.
pub fn preferred_live_ep_labels() -> Vec<&'static str> {
    labels_of(preferred_live_ep_cascade())
}

/// Project the labels of a `(label, EP)` cascade, keeping its order.
fn labels_of(cascade: Vec<(&'static str, ExecutionProviderDispatch)>) -> Vec<&'static str> {
    cascade.into_iter().map(|(label, _)| label).collect()
}

/// `(label, EP)` pairs in preference order (for sequential session tries).
pub(crate) fn preferred_ep_cascade() -> Vec<(&'static str, ExecutionProviderDispatch)> {
    preference_cascade()
}

/// Cascade for the realtime StemgenRT producer: CPU only.
///
/// Measured everywhere: desktop WebGPU is slower than CPU for this graph; on
/// mobile NNAPI 219.6 ms/hop and QNN 13.3–71.5 ms/hop against ~3 ms on CPU
/// (S23), and XNNPACK is within noise of CPU but never faster. A forced EP
/// (`MIXAR_STEMS_ORT_EP` / `debug.mixar.stems_ep`) still wins for experiments.
pub(crate) fn preferred_live_ep_cascade() -> Vec<(&'static str, ExecutionProviderDispatch)> {
    forced_or_else(|| vec![cpu_entry()])
}

fn normalize_ep_key(name: &str) -> String {
    name.trim().to_ascii_lowercase()
}

fn parse_ep_label(name: &str) -> Option<&'static str> {
    match normalize_ep_key(name).as_str() {
        "cpu" | "cpuexecutionprovider" => Some(CPU_EP_LABEL),
        "webgpu" | "webgpuexecutionprovider" => Some(WEBGPU_EP_LABEL),
        // Android only (Microsoft's ORT AAR / custom QNN build). Listed
        // everywhere so the env var is self-documenting; non-Android hosts
        // simply fall back to CPU.
        "nnapi" | "nnapiexecutionprovider" => Some(NNAPI_EP_LABEL),
        "xnnpack" | "xnnpackexecutionprovider" => Some(XNNPACK_EP_LABEL),
        "qnn" | "qnnexecutionprovider" => Some(QNN_EP_LABEL),
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
    forced_or_else(|| {
        let mut out = Vec::new();

        // Desktop offline separation still tries the GPU EP; Android is CPU-only
        // (NNAPI, XNNPACK, QNN and TVM all measured slower for these models).
        #[cfg(any(target_os = "linux", target_os = "windows", target_vendor = "apple"))]
        {
            #[cfg(feature = "webgpu")]
            out.push(ep_entry(WEBGPU_EP_LABEL).expect("webgpu feature"));
        }

        out.push(cpu_entry());
        out
    })
}

/// Honor a forced EP when one is set, otherwise build the platform default.
///
/// Both cascades go through here so the override path cannot drift between the
/// offline and live producers.
fn forced_or_else(
    default: impl FnOnce() -> Vec<(&'static str, ExecutionProviderDispatch)>,
) -> Vec<(&'static str, ExecutionProviderDispatch)> {
    forced_cascade().unwrap_or_else(default)
}

/// Forced single-EP cascade, when `MIXAR_STEMS_ORT_EP` / the Android property
/// names a known provider. Unknown labels fall through to the platform order.
fn forced_cascade() -> Option<Vec<(&'static str, ExecutionProviderDispatch)>> {
    // Reject a label this build cannot build (e.g. `nnapi` on desktop, or
    // `webgpu` without the feature) instead of reporting the label while
    // silently running CPU: fall through to the platform order.
    let label = force_ep_from_env()?;
    Some(vec![ep_entry(label)?])
}

fn cpu_entry() -> (&'static str, ExecutionProviderDispatch) {
    // CPU may remain the only EP; silent failure is fine.
    (CPU_EP_LABEL, ep::CPU::default().build())
}

fn ep_entry(label: &'static str) -> Option<(&'static str, ExecutionProviderDispatch)> {
    // Non-CPU EPs must `.error_on_failure()`: ort's default is fail-silently and
    // still commit a CPU session, which would make our cascade think the GPU EP
    // won while inference stays on CPU.
    let ep = match label {
        CPU_EP_LABEL => ep::CPU::default().build(),
        #[cfg(feature = "webgpu")]
        WEBGPU_EP_LABEL => webgpu_ep().error_on_failure(),
        #[cfg(target_os = "android")]
        NNAPI_EP_LABEL => ep::NNAPI::default().build().error_on_failure(),
        #[cfg(target_os = "android")]
        XNNPACK_EP_LABEL => ep::XNNPACK::default().build().error_on_failure(),
        #[cfg(target_os = "android")]
        QNN_EP_LABEL => qnn_ep(),
        _ => return None,
    };
    Some((label, ep))
}

/// Build one execution provider by label. The cascade and the benchmark tooling
/// share this so env tuning (`MIXAR_QNN_*`) behaves the same in both.
pub fn execution_provider(label: &str) -> Option<ExecutionProviderDispatch> {
    let label = parse_ep_label(label)?;
    ep_entry(label).map(|(_, ep)| ep)
}

/// QNN/HTP needs explicit tuning to be meaningful; defaults mirror ORT (HTP
/// backend, its default performance mode). Overridable per run so the same
/// binary can be benchmarked without rebuilding:
/// `MIXAR_QNN_BACKEND`, `MIXAR_QNN_PERF` (burst/balanced/high/sustained),
/// `MIXAR_QNN_FP16` (0/1), `MIXAR_QNN_HTP_ARCH`.
#[cfg(target_os = "android")]
const QNN_BACKEND_ENV: &str = "MIXAR_QNN_BACKEND";
#[cfg(target_os = "android")]
const QNN_PERF_ENV: &str = "MIXAR_QNN_PERF";
#[cfg(target_os = "android")]
const QNN_FP16_ENV: &str = "MIXAR_QNN_FP16";
#[cfg(target_os = "android")]
const QNN_HTP_ARCH_ENV: &str = "MIXAR_QNN_HTP_ARCH";

#[cfg(target_os = "android")]
fn qnn_ep() -> ExecutionProviderDispatch {
    use ort::ep::qnn::PerformanceMode;

    let mut qnn = ep::QNN::default();
    if let Ok(path) = std::env::var(QNN_BACKEND_ENV) {
        qnn = qnn.with_backend_path(path);
    }
    if let Ok(mode) = std::env::var(QNN_PERF_ENV) {
        qnn = qnn.with_performance_mode(match mode.as_str() {
            "burst" => PerformanceMode::Burst,
            "balanced" => PerformanceMode::Balanced,
            "high" => PerformanceMode::HighPerformance,
            "sustained" => PerformanceMode::SustainedHighPerformance,
            _ => PerformanceMode::Default,
        });
    }
    if let Ok(fp16) = std::env::var(QNN_FP16_ENV) {
        qnn = qnn.with_htp_fp16_precision(fp16 != "0");
    }
    if let Some(arch) = std::env::var(QNN_HTP_ARCH_ENV)
        .ok()
        .and_then(|raw| raw.parse::<u32>().ok())
    {
        qnn = qnn.with_htp_arch(arch);
    }
    qnn.build().error_on_failure()
}

#[cfg(feature = "webgpu")]
fn webgpu_ep() -> ExecutionProviderDispatch {
    // Bare EP registration — with_* helpers pre-prefix `ep.webgpuexecutionprovider.*`,
    // which AppendExecutionProvider prefixes again, so options never apply. Use defaults.
    //
    // No C-locale scope here: this only constructs a dispatch struct, so it does no
    // locale-sensitive ORT work. WGSL float generation happens at commit / first
    // run, which `session::load_session` covers. Wrapping here would also hold
    // the locale lock across a full model load for lock-free pub callers.
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

/// Restores the process-global `LC_NUMERIC` on drop, including on unwind.
#[cfg(all(feature = "webgpu", unix))]
pub(crate) struct LocaleGuard(std::ffi::CString);

#[cfg(all(feature = "webgpu", unix))]
impl LocaleGuard {
    /// Capture the current locale. Call while holding whatever lock serializes
    /// locale access (`ENV_LOCK` in tests, the `SESSION` mutex in production).
    /// Returns `None` when `setlocale(LC_NUMERIC, NULL)` fails — callers
    /// should skip the override rather than risk an unrestorable state.
    pub(crate) fn capture() -> Option<Self> {
        // SAFETY: `setlocale(LC_NUMERIC, NULL)` is the query form — it does
        // not modify process state. The returned pointer is owned by libc and
        // valid only until the next `setlocale`, so it is copied immediately
        // into a `CString` before any further `setlocale` call.
        let current = unsafe { libc::setlocale(libc::LC_NUMERIC, std::ptr::null()) };
        if current.is_null() {
            return None;
        }
        Some(Self(
            unsafe { std::ffi::CStr::from_ptr(current) }.to_owned(),
        ))
    }
}

#[cfg(all(feature = "webgpu", unix))]
impl Drop for LocaleGuard {
    fn drop(&mut self) {
        // SAFETY: `self.0` is an owned `CString` that outlives the call, so
        // the `*const c_char` passed to `setlocale` is valid for its duration.
        // This is process-global mutation; `with_c_numeric_locale` holds
        // `LOCALE_LOCK` around it (tests hold `ENV_LOCK`).
        unsafe {
            libc::setlocale(libc::LC_NUMERIC, self.0.as_ptr());
        }
    }
}

/// Run `f` with `LC_NUMERIC=C`, restoring the previous locale afterwards.
///
/// `LC_NUMERIC` is process-global C state, so the flip is serialized by an
/// internal `LOCALE_LOCK` mutex: concurrent entries cannot interleave a
/// capture/restore over each other's override. `LOCALE_LOCK` is held for the
/// full duration of `f` (necessary — the guarantee only holds while `f` runs),
/// and it is non-reentrant, so no caller may nest one guard inside another.
/// Today the only production caller is `session::load_session` (webgpu only),
/// which the `SESSION` Mutex already serializes; the lock is here so future
/// callers of this `pub(crate)` helper cannot reintroduce the race, not
/// because `preferred_execution_providers()` participates in locale locking.
/// Tests use `ENV_LOCK` for env vars and hold this lock only briefly.
///
/// If the current locale cannot be queried, the override is skipped — `f`
/// still runs, just without the dot-decimal guarantee — rather than risk
/// unrestorable state or panic in a library path.
#[cfg(all(feature = "webgpu", unix))]
pub(crate) fn with_c_numeric_locale<R>(f: impl FnOnce() -> R) -> R {
    static LOCALE_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let _serial = LOCALE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let _restore = LocaleGuard::capture();
    if _restore.is_none() {
        tracing::warn!("stem ort: LC_NUMERIC query failed; running without C-locale override");
    } else {
        force_c_numeric_locale();
    }
    f()
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

    #[cfg(not(target_os = "android"))]
    #[test]
    fn forced_unbuildable_label_falls_through() {
        // `nnapi` parses everywhere but `ep_entry` only builds it on Android;
        // the cascade must not report an EP it cannot run.
        with_force_ep(Some(NNAPI_EP_LABEL), || {
            let labels = preferred_ep_labels();
            assert!(
                !labels.contains(&NNAPI_EP_LABEL),
                "unbuildable forced label must not be reported: {labels:?}"
            );
            assert_eq!(labels.last().copied(), Some(CPU_EP_LABEL));
        });
    }

    #[test]
    fn ep_label_maps_ort_and_short_names() {
        assert_eq!(ep_label("CPUExecutionProvider"), "cpu");
        assert_eq!(ep_label("cuda"), "cuda");
        assert_eq!(ep_label("WebGpuExecutionProvider"), "webgpu");
        assert_eq!(ep_label("DmlExecutionProvider"), "directml");
        assert_eq!(ep_label("NvTensorRTRTXExecutionProvider"), "tensorrt_rtx");
        assert_eq!(ep_label("QnnExecutionProvider"), "qnn");
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
    /// explicitly with `cargo nextest run -p analyzer-stems --run-ignored=only`.
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

    /// The production wrapper must restore the previous `LC_NUMERIC` after the
    /// closure returns. Runs on CI without any special locale: when the ambient
    /// locale is already C the `during` assertion is vacuous but still correct;
    /// when a comma-decimal locale is available it is established first so the
    /// override and the restore both carry signal.
    #[cfg(all(feature = "webgpu", unix))]
    #[test]
    fn c_numeric_locale_is_restored_after_scope() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());

        let (comma_locale, _) = install_first_comma_locale();
        let before = format_float(0.5);
        if let Some(name) = comma_locale {
            assert!(
                before.contains(','),
                "expected {name} to format with a comma, got {before:?}"
            );
        }

        with_c_numeric_locale(|| {
            let during = format_float(0.5);
            assert!(
                during.contains('.') && !during.contains(','),
                "scoped override must format with a dot, got {during:?} \
                 (precondition comma locale: {comma_locale:?}, before: {before:?})"
            );
        });
        let after = format_float(0.5);
        assert_eq!(
            before, after,
            "LC_NUMERIC was not restored after with_c_numeric_locale: before={before:?}, after={after:?}"
        );
    }
}
