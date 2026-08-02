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
