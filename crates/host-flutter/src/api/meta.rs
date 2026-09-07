//! Host metadata exposed to Flutter (shared branding).

/// Product display name ([`engine_api::APP_DISPLAY_NAME`]).
#[flutter_rust_bridge::frb(sync)]
pub fn app_display_name() -> String {
    engine_api::APP_DISPLAY_NAME.to_string()
}

#[flutter_rust_bridge::frb(init)]
pub fn init_app() {
    init_tracing();
}

fn init_tracing() {
    use tracing_subscriber::EnvFilter;

    // Bridge transitive `log` users (e.g. FRB helpers) into the same subscriber.
    let _ = tracing_log::LogTracer::init();

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let subscriber = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .finish();
    let _ = tracing::subscriber::set_global_default(subscriber);
}
