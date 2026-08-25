//! Trusted device auto-attach vs MappingOffer (no successful MIDI open required).

use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

use controller::{ControllerEngine, ControllerEvent};
use tempfile::TempDir;

fn write_minimal_bundle(dir: &PathBuf, id: &str, product_name: &str, midi_needle: &str) {
    fs::create_dir_all(dir).unwrap();
    fs::write(
        dir.join("device.toml"),
        format!(
            r#"schema_version = 1
id = "{id}"
vendor_name = "Test"
product_name = "{product_name}"
midi_name_contains = ["{midi_needle}"]

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

struct Fixture {
    _root: TempDir,
    engine: ControllerEngine,
}

fn open_with_map() -> Fixture {
    let root = tempfile::tempdir().unwrap();
    let shipped = root.path().join("shipped");
    let app = root.path().join("app");
    write_minimal_bundle(&shipped.join("test-map"), "test.map", "Test Map", "TestDev");
    let engine = ControllerEngine::open("test", &app, &shipped).unwrap();
    Fixture {
        _root: root,
        engine,
    }
}

fn open_with_two_maps() -> Fixture {
    let root = tempfile::tempdir().unwrap();
    let shipped = root.path().join("shipped");
    let app = root.path().join("app");
    write_minimal_bundle(&shipped.join("test-map"), "test.map", "Test Map", "TestDev");
    write_minimal_bundle(
        &shipped.join("other-map"),
        "other.map",
        "Other Map",
        "OtherDev",
    );
    let engine = ControllerEngine::open("test", &app, &shipped).unwrap();
    Fixture {
        _root: root,
        engine,
    }
}

#[test]
fn untrusted_new_port_emits_offer_with_device_id() {
    let mut fx = open_with_map();
    fx.engine
        .apply_input_ports(HashSet::from(["TestDev Port".into()]));
    let ev = fx.engine.take_events();
    assert!(
        matches!(
            &ev[..],
            [ControllerEvent::MappingOffer {
                mapping_id,
                device_id,
                port_name,
                ..
            }] if mapping_id == "test-map"
                && device_id == "test.map"
                && port_name == "TestDev Port"
        ),
        "{ev:?}"
    );
}

#[test]
fn trusted_new_port_does_not_emit_offer() {
    let mut fx = open_with_map();
    fx.engine.set_trusted_device_ids(["test.map".into()]);
    fx.engine
        .apply_input_ports(HashSet::from(["TestDev Port".into()]));
    let ev = fx.engine.take_events();
    assert!(
        ev.iter()
            .all(|e| !matches!(e, ControllerEvent::MappingOffer { .. })),
        "{ev:?}"
    );
}

#[test]
fn multiple_trusted_ids_skip_offers() {
    let mut fx = open_with_two_maps();
    fx.engine
        .set_trusted_device_ids(["test.map".into(), "other.map".into()]);
    fx.engine.apply_input_ports(HashSet::from([
        "TestDev Port".into(),
        "OtherDev Port".into(),
    ]));
    let ev = fx.engine.take_events();
    assert!(
        ev.iter()
            .all(|e| !matches!(e, ControllerEvent::MappingOffer { .. })),
        "{ev:?}"
    );
}

#[test]
fn pending_offers_omit_trusted_matches() {
    let mut fx = open_with_map();
    fx.engine.set_trusted_device_ids(["test.map".into()]);
    fx.engine
        .apply_input_ports(HashSet::from(["TestDev Port".into()]));
    assert!(fx.engine.pending_offers().is_empty());
}
