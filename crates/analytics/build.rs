use std::fs;

fn main() {
    const PUBSPEC: &str = "../../apps/gui-flutter/pubspec.yaml";
    println!("cargo:rerun-if-changed={PUBSPEC}");

    let pubspec = fs::read_to_string(PUBSPEC).expect("read Flutter pubspec for the app version");
    let version = pubspec
        .lines()
        .find_map(|line| line.strip_prefix("version: "))
        .and_then(|version| version.split('+').next())
        .map(str::trim)
        .filter(|version| !version.is_empty())
        .expect("Flutter pubspec must declare a version");
    println!("cargo:rustc-env=MIXAR_APP_VERSION={version}");
}
