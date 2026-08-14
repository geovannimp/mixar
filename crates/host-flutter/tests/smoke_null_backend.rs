//! Smoke checks for Flutter host audio APIs (null backend — no real audio device).

use host_flutter::api::engine::AudioBackendTransport;

#[test]
fn list_backends_includes_null_and_auto() {
    let names = AudioBackendTransport::list_names();
    assert!(
        names.iter().any(|n| n == "null"),
        "expected null backend in {names:?}"
    );
    assert_eq!(names.first().map(String::as_str), Some("auto"));
}

#[test]
fn list_null_devices() {
    let backend = AudioBackendTransport::open("null".into()).unwrap();
    let devices = backend.list_output_devices().unwrap();
    assert!(!devices.is_empty());
}
