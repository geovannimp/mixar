use std::path::Path;

#[test]
fn loads_valid_minimal_bundle() {
    let b = controller::load_bundle(Path::new("tests/fixtures/valid-minimal")).unwrap();
    assert_eq!(b.device.id, "test.minimal");
}

#[test]
fn rejects_unknown_input_alias() {
    let err =
        controller::load_bundle(Path::new("tests/fixtures/invalid-unknown-alias")).unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("not_a_real_alias") || msg.contains("closed catalog"),
        "unexpected error: {msg}"
    );
}

#[test]
fn parses_toml_schema_metadata_and_ignores_it() {
    // `[toml-schema]` must follow root scalars — TOML attaches following keys to the open table.
    let device = controller::DeviceFile::parse(
        r#"
schema_version = 1
id = "test.schema"
vendor_name = "Test"
product_name = "Schema"

[toml-schema]
version = "1.0.0"
location = "../../schemas/device.tosd"

[deck_1]
play_pause = { type = "note", channel = 1, note = 0x0B }
"#,
        Path::new("device.toml"),
    )
    .unwrap();
    assert_eq!(device.id, "test.schema");
    let meta = device.toml_schema.expect("toml-schema present");
    assert_eq!(meta.version.as_deref(), Some("1.0.0"));
    assert_eq!(meta.location.as_deref(), Some("../../schemas/device.tosd"));

    let map = controller::MapFile::parse(
        r#"
schema_version = 1

[toml-schema]
version = "1.0.0"
location = "../../schemas/map.tosd"

[inputs.deck_1]
play_pause = "Deck(_)::toggle_play"
"#,
        Path::new("map.toml"),
    )
    .unwrap();
    let meta = map.toml_schema.expect("toml-schema present");
    assert_eq!(meta.location.as_deref(), Some("../../schemas/map.tosd"));
}

