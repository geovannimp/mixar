//! Host metadata exposed to Flutter (shared branding).

/// Product display name — same string Tauri / other hosts should use ([`engine_api::APP_DISPLAY_NAME`]).
#[flutter_rust_bridge::frb(sync)]
pub fn app_display_name() -> String {
    engine_api::APP_DISPLAY_NAME.to_string()
}

#[flutter_rust_bridge::frb(init)]
pub fn init_app() {
    flutter_rust_bridge::setup_default_user_utils();
}
