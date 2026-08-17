//! ControllerTransport: seed catalog from shipped mappings (no MIDI hardware).

use std::fs;
use std::path::PathBuf;

use engine_core::EngineBuses;
use host_flutter::api::controller::ControllerTransport;
use host_flutter::api::engine::EngineBusHandle;
use host_flutter::api::library::LibraryBusHandle;
use library::LibraryBuses;

fn write_minimal_bundle(dir: &PathBuf, id: &str, product_name: &str) {
    fs::create_dir_all(dir).unwrap();
    fs::write(
        dir.join("device.toml"),
        format!(
            r#"schema_version = 1
id = "{id}"
vendor_name = "Test"
product_name = "{product_name}"
midi_name_contains = ["TestDev"]

[toml-schema]
version = "1.0.0"
location = "../../../../schemas/device.tosd"

[deck_1]
play_pause = {{ type = "note", channel = 1, note = 0x0B }}
"#
        ),
    )
    .unwrap();
    fs::write(
        dir.join("map.toml"),
        r#"schema_version = 1

[toml-schema]
version = "1.0.0"
location = "../../../../schemas/map.tosd"

[inputs.deck_1]
play_pause = "Deck(_)::toggle_play"
"#,
    )
    .unwrap();
}

#[test]
fn start_seeds_and_lists_shipped_mapping() {
    let root = tempfile::tempdir().unwrap();
    let shipped = root.path().join("shipped");
    let app = root.path().join("app");
    write_minimal_bundle(&shipped.join("test-map"), "test.map", "Test Map");

    let transport = ControllerTransport::start(
        &EngineBusHandle::from_buses(EngineBuses::new()),
        &LibraryBusHandle::from_buses(LibraryBuses::new()),
        app.to_string_lossy().into_owned(),
        Some(shipped.to_string_lossy().into_owned()),
    )
    .unwrap();

    let mappings = transport.list_mappings().unwrap();
    assert_eq!(mappings.len(), 1);
    assert_eq!(mappings[0].id, "test-map");
    assert_eq!(mappings[0].product_name, "Test Map");
    assert!(!mappings[0].attached);
}
