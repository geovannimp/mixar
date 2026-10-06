//! Static mapping-bundle validation — the contract `map-check` relies on.
//!
//! `rust:test-mappings` was previously never scheduled by CI (`moon run :test`
//! only matches tasks named `test`), so the shipped bundles went unvalidated.

use std::fs;
use std::path::{Path, PathBuf};

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn copy_dir(src: &Path, dst: &Path) {
    fs::create_dir_all(dst).expect("create dir");
    for entry in fs::read_dir(src).expect("read dir") {
        let entry = entry.expect("dir entry");
        let to = dst.join(entry.file_name());
        if entry.path().is_dir() {
            copy_dir(&entry.path(), &to);
        } else {
            fs::copy(entry.path(), &to).expect("copy file");
        }
    }
}

#[test]
fn shipped_mappings_are_valid() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../mappings");
    let ok = controller::check::check_all_mappings(&root)
        .unwrap_or_else(|errs| panic!("shipped bundles must validate, got {errs:?}"));
    assert!(
        !ok.is_empty(),
        "expected at least one shipped mapping bundle, found none"
    );
}

#[test]
fn empty_mappings_root_is_an_error() {
    let dir = tempfile::tempdir().expect("tempdir");
    let err = controller::check::check_all_mappings(dir.path())
        .expect_err("an empty mappings root must not pass silently");
    assert!(
        err.iter()
            .any(|(_, e)| format!("{e}").contains("no mapping bundle")),
        "expected a missing-bundle error, got {err:?}"
    );
}

#[test]
fn typo_in_script_binding_is_rejected() {
    let dir = tempfile::tempdir().expect("tempdir");
    let bundle = dir.path().join("bundle");
    copy_dir(&fixture("with-script"), &bundle);

    // Keep the alias/endpoint valid so validation reaches the script check, and
    // point it at a function the script does not define.
    let map = bundle.join("map.toml");
    let text = fs::read_to_string(&map).expect("read map.toml");
    let text = text.replace(
        "play_pause = \"Deck(_)::toggle_play\"",
        "play_pause = { script = \"totally_missing_fn\" }",
    );
    fs::write(&map, text).expect("write map.toml");

    let err = controller::check_bundle_dir(&bundle)
        .expect_err("a script binding naming a missing function must be rejected");
    let message = format!("{err}");
    assert!(
        message.contains("totally_missing_fn"),
        "error must name the missing function, got: {message}"
    );
}

#[test]
fn valid_bundle_with_script_passes() {
    controller::check_bundle_dir(&fixture("with-script"))
        .expect("the with-script fixture must validate");
}
