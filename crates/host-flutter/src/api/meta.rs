//! Host metadata exposed to Flutter (shared branding).

use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use tracing_subscriber::fmt::MakeWriter;
use tracing_subscriber::EnvFilter;

/// Product display name ([`engine_api::APP_DISPLAY_NAME`]).
#[flutter_rust_bridge::frb(sync)]
pub fn app_display_name() -> String {
    engine_api::APP_DISPLAY_NAME.to_string()
}

#[flutter_rust_bridge::frb(init)]
pub fn init_app() {
    init_tracing();
    // Keep FRB panic backtraces (`RUST_BACKTRACE` + hook); skip its `log` console init.
    flutter_rust_bridge::setup_backtrace();
}

fn tracing_env_filter() -> EnvFilter {
    EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"))
}

/// stderr + optional app-support file (`mixar.log`).
struct TeeState {
    file: Option<File>,
}

fn tee_state() -> &'static Mutex<TeeState> {
    static STATE: OnceLock<Mutex<TeeState>> = OnceLock::new();
    STATE.get_or_init(|| Mutex::new(TeeState { file: None }))
}

#[derive(Clone, Copy)]
struct MakeTee;

struct TeeWriter;

impl Write for TeeWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let _ = io::stderr().write_all(buf);
        if let Ok(mut state) = tee_state().lock() {
            if let Some(file) = state.file.as_mut() {
                let _ = file.write_all(buf);
            }
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        let _ = io::stderr().flush();
        if let Ok(mut state) = tee_state().lock() {
            if let Some(file) = state.file.as_mut() {
                let _ = file.flush();
            }
        }
        Ok(())
    }
}

impl<'a> MakeWriter<'a> for MakeTee {
    type Writer = TeeWriter;
    fn make_writer(&'a self) -> Self::Writer {
        TeeWriter
    }
}

fn init_tracing() {
    // Bridge transitive `log` users (e.g. FRB helpers) into the same subscriber.
    let _ = tracing_log::LogTracer::init();

    let subscriber = tracing_subscriber::fmt()
        .with_env_filter(tracing_env_filter())
        .with_writer(MakeTee)
        .finish();
    let _ = tracing::subscriber::set_global_default(subscriber);
}

/// Append Mixar diagnostics to `path` (typically `{app-support}/mixar.log`).
///
/// Idempotent: later calls replace the open handle. Safe to call after [`init_app`].
pub fn attach_app_log_file(path: impl AsRef<Path>) {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        if let Err(err) = fs::create_dir_all(parent) {
            tracing::warn!(
                path = %path.display(),
                error = %err,
                "failed to create application log directory"
            );
            return;
        }
    }
    match OpenOptions::new().create(true).append(true).open(path) {
        Ok(file) => {
            if let Ok(mut state) = tee_state().lock() {
                state.file = Some(file);
            }
            tracing::info!(path = %path.display(), "application log file attached");
        }
        Err(err) => {
            tracing::warn!(
                path = %path.display(),
                error = %err,
                "failed to open application log file"
            );
        }
    }
}

/// `{mappings_dir}/../mixar.log` when `mappings_dir` is `{app-support}/mappings`.
pub fn attach_app_log_beside_mappings(mappings_dir: impl AsRef<Path>) {
    let mappings_dir = mappings_dir.as_ref();
    let path = mappings_dir
        .parent()
        .map(|p| p.join("mixar.log"))
        .unwrap_or_else(|| PathBuf::from("mixar.log"));
    attach_app_log_file(path);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, OnceLock};

    fn rust_log_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    fn with_rust_log(value: Option<&str>, f: impl FnOnce()) {
        let _guard = rust_log_lock().lock().unwrap();
        let prev = std::env::var_os("RUST_LOG");
        match value {
            Some(v) => std::env::set_var("RUST_LOG", v),
            None => std::env::remove_var("RUST_LOG"),
        }
        f();
        match prev {
            Some(v) => std::env::set_var("RUST_LOG", v),
            None => std::env::remove_var("RUST_LOG"),
        }
    }

    #[test]
    fn tracing_env_filter_defaults_to_info_when_unset() {
        with_rust_log(None, || {
            assert_eq!(tracing_env_filter().to_string(), "info");
        });
    }

    #[test]
    fn tracing_env_filter_defaults_to_info_when_invalid() {
        with_rust_log(Some("%%%"), || {
            assert_eq!(tracing_env_filter().to_string(), "info");
        });
    }

    #[test]
    fn tracing_env_filter_respects_rust_log_debug() {
        with_rust_log(Some("debug"), || {
            assert_eq!(tracing_env_filter().to_string(), "debug");
        });
    }

    #[test]
    fn attach_app_log_file_receives_tracing_events() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mixar.log");
        let subscriber = tracing_subscriber::fmt()
            .with_max_level(tracing::Level::INFO)
            .with_writer(MakeTee)
            .with_ansi(false)
            .finish();
        tracing::subscriber::with_default(subscriber, || {
            attach_app_log_file(&path);
            tracing::error!(
                mapping_id = "ddj-400",
                hook = "on_init",
                "script runtime error: Variable not found: PC_MODE_SYSEX (line 10, position 14)"
            );
        });
        let text = fs::read_to_string(&path).unwrap();
        assert!(text.contains("PC_MODE_SYSEX"), "{text}");
        assert!(text.contains("on_init"), "{text}");
        assert!(text.contains("ddj-400"), "{text}");
    }
}
