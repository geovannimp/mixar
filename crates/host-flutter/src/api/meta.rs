//! Host metadata exposed to Flutter (shared branding).

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

fn init_tracing() {
    // Bridge transitive `log` users (e.g. FRB helpers) into the same subscriber.
    let _ = tracing_log::LogTracer::init();

    let subscriber = tracing_subscriber::fmt()
        .with_env_filter(tracing_env_filter())
        .with_writer(std::io::stderr)
        .finish();
    let _ = tracing::subscriber::set_global_default(subscriber);
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
}
