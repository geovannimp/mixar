use std::fmt::Display;
use std::fs;
use std::path::Path;

mod build_support;

fn fail(context: &str, path: &Path, error: impl Display) -> ! {
    panic!("{context} Flutter pubspec {}: {error}", path.display());
}

fn main() {
    let pubspec_path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../apps/gui-flutter/pubspec.yaml");
    println!("cargo:rerun-if-changed={}", pubspec_path.display());

    let pubspec = fs::read_to_string(&pubspec_path)
        .unwrap_or_else(|error| fail("failed to read", &pubspec_path, error));
    let version = build_support::app_version(&pubspec)
        .unwrap_or_else(|error| fail("invalid app version in", &pubspec_path, error));
    println!("cargo:rustc-env=MIXAR_APP_VERSION={version}");
}
