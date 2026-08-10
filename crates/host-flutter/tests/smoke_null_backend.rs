//! Smoke checks for the Flutter host API (null backend — no real audio device).

#[test]
fn list_backends_includes_null_and_auto() {
    let names = host_flutter::api::engine::list_backend_names();
    assert!(
        names.iter().any(|n| n == "null"),
        "expected null backend in {names:?}"
    );
    assert_eq!(names.first().map(String::as_str), Some("auto"));
}

#[test]
fn start_stop_null_backend() {
    // Ensure clean slate if another test left a session (same process).
    let _ = host_flutter::api::engine::stop_engine();
    assert!(!host_flutter::api::engine::engine_is_running());

    host_flutter::api::engine::start_engine("null".into(), None, None).unwrap();
    assert!(host_flutter::api::engine::engine_is_running());

    // Idempotent
    host_flutter::api::engine::start_engine("null".into(), None, None).unwrap();
    assert!(host_flutter::api::engine::engine_is_running());

    host_flutter::api::engine::stop_engine().unwrap();
    assert!(!host_flutter::api::engine::engine_is_running());
}

#[test]
fn list_null_devices() {
    let devices = host_flutter::api::engine::list_output_devices("null".into()).unwrap();
    assert!(!devices.is_empty());
}
