//! Embed the app version the Flutter manifest declares.
//!
//! `apps/gui-flutter/pubspec.yaml` is what releases bump, and what packaging
//! and the About surface read; the workspace crates carry their own version.
//! Reading the manifest here keeps the version reported with analytics events
//! in step with a pubspec-only bump.

use std::path::PathBuf;

fn main() {
    let pubspec =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../apps/gui-flutter/pubspec.yaml");
    println!("cargo:rerun-if-changed={}", pubspec.display());

    let text = std::fs::read_to_string(&pubspec)
        .unwrap_or_else(|error| panic!("read {}: {error}", pubspec.display()));
    let version = text
        .lines()
        .find_map(|line| line.strip_prefix("version: "))
        .unwrap_or_else(|| panic!("no `version:` line in {}", pubspec.display()))
        .trim();
    println!("cargo:rustc-env=MIXAR_APP_VERSION={version}");
}
