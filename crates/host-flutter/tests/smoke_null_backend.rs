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
fn list_null_devices_maps_the_bridge_contract() {
    let backend = AudioBackendTransport::open("null".into()).unwrap();
    let devices = backend.list_output_devices().unwrap();

    // Assert the bridge contract — the DTO mapping is populated and a default
    // device is surfaced — rather than the null backend's exact identity. Exact
    // counts/names are asserted by `backend-null`'s own tests, so renaming or
    // adding a null device should not break this host test.
    assert!(!devices.is_empty(), "null backend must expose a device");
    assert!(
        devices.iter().any(|d| d.is_default),
        "one device must be flagged default"
    );
    for device in &devices {
        assert!(
            !device.id.is_empty(),
            "device id must map through: {device:?}"
        );
        assert!(
            !device.name.is_empty(),
            "device name must map through: {device:?}"
        );
        assert!(
            device.max_channels > 0,
            "channels must map through: {device:?}"
        );
    }
}
