//! ControllerEngine seed / update filesystem behavior (no MIDI hardware).

use std::fs;
use std::path::PathBuf;

use controller::ControllerEngine;

fn write_minimal_bundle(dir: &PathBuf, id: &str, product_name: &str) {
    write_bundle(dir, id, product_name, None);
}

fn write_bundle(dir: &PathBuf, id: &str, product_name: &str, version: Option<&str>) {
    fs::create_dir_all(dir).unwrap();
    let version_line = version
        .map(|v| format!("mapping_version = \"{v}\"\n"))
        .unwrap_or_default();
    fs::write(
        dir.join("device.toml"),
        format!(
            r#"schema_version = 1
id = "{id}"
vendor_name = "Test"
product_name = "{product_name}"
{version_line}midi_name_contains = ["TestDev"]

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

/// Shipped version newer than installed → `update_available`.
#[test]
fn update_available_when_shipped_is_newer() {
    let root = tempfile::tempdir().unwrap();
    let shipped = root.path().join("shipped");
    let app = root.path().join("app");
    write_bundle(
        &shipped.join("test-map"),
        "test.map",
        "Test Map",
        Some("1.1.0"),
    );
    write_bundle(&app.join("test-map"), "test.map", "Test Map", Some("1.0.0"));

    let engine = ControllerEngine::open("test", &app, &shipped).unwrap();
    let list = engine.list_mappings().unwrap();
    assert_eq!(list[0].version.as_deref(), Some("1.0.0"));
    assert!(list[0].update_available);
}

/// Equal versions → no update, even if other bundle fields differ.
#[test]
fn no_update_when_versions_match() {
    let root = tempfile::tempdir().unwrap();
    let shipped = root.path().join("shipped");
    let app = root.path().join("app");
    write_bundle(
        &shipped.join("test-map"),
        "test.map",
        "Shipped Name",
        Some("1.0.0"),
    );
    write_bundle(
        &app.join("test-map"),
        "test.map",
        "Local Name",
        Some("1.0.0"),
    );

    let engine = ControllerEngine::open("test", &app, &shipped).unwrap();
    let list = engine.list_mappings().unwrap();
    assert_eq!(list[0].product_name, "Local Name");
    assert!(!list[0].update_available);
}

/// Installed newer than shipped (e.g. a local dev bundle) → no downgrade.
#[test]
fn no_update_when_installed_is_newer() {
    let root = tempfile::tempdir().unwrap();
    let shipped = root.path().join("shipped");
    let app = root.path().join("app");
    write_bundle(
        &shipped.join("test-map"),
        "test.map",
        "Test Map",
        Some("1.0.0"),
    );
    write_bundle(&app.join("test-map"), "test.map", "Test Map", Some("2.0.0"));

    let engine = ControllerEngine::open("test", &app, &shipped).unwrap();
    let list = engine.list_mappings().unwrap();
    assert!(!list[0].update_available);
}

/// A legacy installed bundle with no version is updateable once shipped declares one.
#[test]
fn update_available_for_unversioned_installed() {
    let root = tempfile::tempdir().unwrap();
    let shipped = root.path().join("shipped");
    let app = root.path().join("app");
    write_bundle(
        &shipped.join("test-map"),
        "test.map",
        "Test Map",
        Some("1.0.0"),
    );
    write_minimal_bundle(&app.join("test-map"), "test.map", "Test Map");

    let engine = ControllerEngine::open("test", &app, &shipped).unwrap();
    let list = engine.list_mappings().unwrap();
    assert_eq!(list[0].version, None);
    assert!(list[0].update_available);
}

/// Unversioned shipped bundle → nothing to compare, never flagged.
#[test]
fn no_update_when_shipped_unversioned() {
    let root = tempfile::tempdir().unwrap();
    let shipped = root.path().join("shipped");
    let app = root.path().join("app");
    write_minimal_bundle(&shipped.join("test-map"), "test.map", "Test Map");
    write_minimal_bundle(&app.join("test-map"), "test.map", "Test Map");

    let engine = ControllerEngine::open("test", &app, &shipped).unwrap();
    let list = engine.list_mappings().unwrap();
    assert!(!list[0].update_available);
}

/// Updating an outdated mapping clears the flag (installed becomes shipped version).
#[test]
fn update_mapping_clears_update_available() {
    let root = tempfile::tempdir().unwrap();
    let shipped = root.path().join("shipped");
    let app = root.path().join("app");
    write_bundle(
        &shipped.join("test-map"),
        "test.map",
        "Test Map",
        Some("1.1.0"),
    );
    write_bundle(&app.join("test-map"), "test.map", "Test Map", Some("1.0.0"));

    let mut engine = ControllerEngine::open("test", &app, &shipped).unwrap();
    assert!(engine.list_mappings().unwrap()[0].update_available);

    engine.update_mapping("test-map").unwrap();
    let list = engine.list_mappings().unwrap();
    assert_eq!(list[0].version.as_deref(), Some("1.1.0"));
    assert!(!list[0].update_available);
}
