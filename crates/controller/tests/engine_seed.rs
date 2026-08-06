//! ControllerEngine seed / update filesystem behavior (no MIDI hardware).

use std::fs;
use std::path::PathBuf;

use controller::ControllerEngine;

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

[deck_1]
play_pause = {{ type = "note", channel = 1, note = 0x0B }}
"#
        ),
    )
    .unwrap();
    fs::write(
        dir.join("map.toml"),
        r#"schema_version = 1

[inputs.deck_1]
play_pause = "Deck(_)::toggle_play"
"#,
    )
    .unwrap();
}

#[test]
fn ensure_seeded_copies_missing_only() {
    let root = tempfile::tempdir().unwrap();
    let shipped = root.path().join("shipped");
    let app = root.path().join("app");
    let id_dir = shipped.join("test-map");
    write_minimal_bundle(&id_dir, "test.map", "Test Map");

    let mut engine = ControllerEngine::open("test", &app, &shipped).unwrap();
    assert!(app.join("test-map").join("device.toml").is_file());

    // Mutate app-data; seed again must not overwrite.
    fs::write(app.join("test-map").join("marker"), "keep").unwrap();
    engine.ensure_seeded().unwrap();
    assert_eq!(
        fs::read_to_string(app.join("test-map").join("marker")).unwrap(),
        "keep"
    );
}

#[test]
fn update_mapping_overwrites_app_data() {
    let root = tempfile::tempdir().unwrap();
    let shipped = root.path().join("shipped");
    let app = root.path().join("app");
    write_minimal_bundle(&shipped.join("test-map"), "test.map", "Test Map");

    let mut engine = ControllerEngine::open("test", &app, &shipped).unwrap();
    fs::write(app.join("test-map").join("marker"), "old").unwrap();

    // Change shipped name and update.
    write_minimal_bundle(&shipped.join("test-map"), "test.map", "Updated Map");
    engine.update_mapping("test-map").unwrap();
    assert!(!app.join("test-map").join("marker").exists());
    let list = engine.list_mappings().unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].id, "test-map");
    assert_eq!(list[0].product_name, "Updated Map");
}
