//! Install pinned `betterhook-cli` into `tools/` and run `betterhook install`.
//! Failures warn only — they must not break `cargo build`.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const BETTERHOOK_CLI_VERSION: &str = "0.1.0";

fn main() {
    println!("cargo:rerun-if-env-changed=CI");
    println!("cargo:rerun-if-env-changed=BETTERHOOK_BOOTSTRAP");
    println!("cargo:rerun-if-changed=../betterhook.toml");

    if env::var_os("CI").is_some() {
        return;
    }
    if env::var("BETTERHOOK_BOOTSTRAP").ok().as_deref() == Some("0") {
        return;
    }

    let Some(workspace_root) = workspace_root() else {
        return;
    };
    if !workspace_root.join(".git").exists() {
        return;
    }
    if !workspace_root.join("betterhook.toml").is_file() {
        println!("cargo:warning=hooks-install: betterhook.toml missing; skipping bootstrap");
        return;
    }

    let tools_dir = workspace_root.join("tools");
    let betterhook_bin = tools_dir.join("bin").join("betterhook");
    let marker = tools_dir.join(".betterhook-bootstrapped");

    if betterhook_bin.is_file()
        && fs::read_to_string(&marker)
            .ok()
            .is_some_and(|v| v.trim() == BETTERHOOK_CLI_VERSION)
    {
        return;
    }

    if let Err(err) = bootstrap(&workspace_root, &tools_dir, &betterhook_bin, &marker) {
        println!("cargo:warning=hooks-install: {err}");
    }
}

fn workspace_root() -> Option<PathBuf> {
    let manifest_dir = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR")?);
    Some(manifest_dir.parent()?.to_path_buf())
}

fn bootstrap(
    workspace_root: &Path,
    tools_dir: &Path,
    betterhook_bin: &Path,
    marker: &Path,
) -> Result<(), String> {
    fs::create_dir_all(tools_dir).map_err(|e| format!("create tools/: {e}"))?;

    let cargo = env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let install = Command::new(&cargo)
        .args([
            "install",
            "betterhook-cli",
            "--version",
            BETTERHOOK_CLI_VERSION,
            "--root",
        ])
        .arg(tools_dir)
        .args(["--locked", "--force"])
        .current_dir(workspace_root)
        .status()
        .map_err(|e| format!("spawn cargo install: {e}"))?;

    if !install.success() {
        return Err(format!(
            "cargo install betterhook-cli@{BETTERHOOK_CLI_VERSION} failed (status {install})"
        ));
    }

    if !betterhook_bin.is_file() {
        return Err(format!(
            "betterhook binary missing after install: {}",
            betterhook_bin.display()
        ));
    }

    let install_hooks = Command::new(betterhook_bin)
        .args(["install", "--no-unit"])
        .current_dir(workspace_root)
        .status()
        .map_err(|e| format!("spawn betterhook install: {e}"))?;

    if !install_hooks.success() {
        return Err(format!(
            "betterhook install failed (status {install_hooks})"
        ));
    }

    fs::write(marker, format!("{BETTERHOOK_CLI_VERSION}\n"))
        .map_err(|e| format!("write marker: {e}"))?;

    Ok(())
}
