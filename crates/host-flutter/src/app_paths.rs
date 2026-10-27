//! App-support directory and the files Mixar keeps in it, resolved in Rust.
//!
//! Dart used to resolve these with `path_provider` and hand them across the bridge
//! (a platform-channel round trip per transport), which also meant nothing could be
//! logged until Dart called in. The rules below reproduce exactly what
//! `path_provider` derives per host, so installs keep finding their existing
//! `library.db`, `settings.json` and `mixar.log`:
//!
//! | host    | directory                                                   |
//! |---------|-------------------------------------------------------------|
//! | Linux   | `$XDG_DATA_HOME/top.mixar.app` or `~/.local/share/…`         |
//! | macOS   | `~/Library/Application Support/top.mixar.app`                |
//! | Windows | `%APPDATA%\Mixar`                                            |

use std::path::{Path, PathBuf};

/// Directory leaf on Linux and macOS: `APPLICATION_ID` / `PRODUCT_BUNDLE_IDENTIFIER`.
const APP_ID: &str = "top.mixar.app";

/// Directory leaf on Windows. `path_provider_windows` joins the executable's
/// `CompanyName` and `ProductName` VERSIONINFO fields; `windows/runner/Runner.rc`
/// leaves `CompanyName` empty, so the product name alone names the directory.
const WINDOWS_PRODUCT_NAME: &str = "Mixar";

/// Host whose path rules apply. Passed in so every rule is testable everywhere.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Host {
    Linux,
    MacOs,
    Windows,
}

const fn current_host() -> Host {
    if cfg!(target_os = "windows") {
        Host::Windows
    } else if cfg!(target_os = "macos") {
        Host::MacOs
    } else {
        Host::Linux
    }
}

fn env_path(key: &str) -> Option<PathBuf> {
    std::env::var_os(key)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

/// Resolve the app-support directory for `host` from the ambient environment.
///
/// A relative `XDG_DATA_HOME` is ignored, as the XDG spec and `path_provider_linux`
/// do. With no base directory at all (a daemon-ish launch with no `HOME`/`APPDATA`)
/// it falls back to the working directory rather than producing an unanchored path.
fn support_dir(
    host: Host,
    home: Option<&Path>,
    xdg_data_home: Option<&Path>,
    app_data: Option<&Path>,
) -> PathBuf {
    let base = match host {
        Host::Linux => xdg_data_home
            .filter(|dir| dir.is_absolute())
            .map(Path::to_path_buf)
            .or_else(|| home.map(|home| home.join(".local/share"))),
        Host::MacOs => home.map(|home| home.join("Library/Application Support")),
        Host::Windows => app_data.map(Path::to_path_buf),
    }
    .unwrap_or_else(|| PathBuf::from("."));

    let leaf = match host {
        Host::Linux | Host::MacOs => APP_ID,
        Host::Windows => WINDOWS_PRODUCT_NAME,
    };
    base.join(leaf)
}

/// `{app-support}` on the running host (not created).
pub(crate) fn support() -> PathBuf {
    let home = env_path("HOME").or_else(|| env_path("USERPROFILE"));
    let xdg_data_home = env_path("XDG_DATA_HOME");
    let app_data = env_path("APPDATA");
    support_dir(
        current_host(),
        home.as_deref(),
        xdg_data_home.as_deref(),
        app_data.as_deref(),
    )
}

/// `{app-support}`, created if missing. Call once at start-up so every later path
/// (log included) has a directory to live in.
pub(crate) fn ensure() -> PathBuf {
    let dir = support();
    if let Err(error) = std::fs::create_dir_all(&dir) {
        // The log directory is the one failure that cannot be reported into the log.
        eprintln!(
            "mixar: cannot create app-support directory {}: {error}",
            dir.display()
        );
    }
    dir
}

/// `{app-support}/library.db` — catalog, tags, analysis, waveform overviews.
pub(crate) fn library_db() -> PathBuf {
    support().join("library.db")
}

/// `{app-support}/settings.json` — every preference.
pub(crate) fn settings_json() -> PathBuf {
    support().join("settings.json")
}

/// `{app-support}/mixar.log` — Rust diagnostics, attached at start-up.
pub(crate) fn log_file() -> PathBuf {
    support().join("mixar.log")
}

/// `{app-support}/mappings` — controller mapping bundles, seeded from the shipped set.
pub(crate) fn mappings_dir() -> PathBuf {
    support().join("mappings")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linux_prefers_xdg_data_home_and_falls_back_to_local_share() {
        let home = Path::new("/home/mix");
        assert_eq!(
            support_dir(Host::Linux, Some(home), Some(Path::new("/data")), None),
            Path::new("/data/top.mixar.app")
        );
        assert_eq!(
            support_dir(Host::Linux, Some(home), None, None),
            Path::new("/home/mix/.local/share/top.mixar.app")
        );
        // A relative XDG_DATA_HOME is not a valid XDG base directory.
        assert_eq!(
            support_dir(Host::Linux, Some(home), Some(Path::new("relative")), None),
            Path::new("/home/mix/.local/share/top.mixar.app")
        );
    }

    #[test]
    fn macos_uses_application_support_under_home() {
        assert_eq!(
            support_dir(Host::MacOs, Some(Path::new("/Users/mix")), None, None),
            Path::new("/Users/mix/Library/Application Support/top.mixar.app")
        );
    }

    #[test]
    fn windows_uses_roaming_app_data_and_the_product_name() {
        // The product name alone: `CompanyName` is empty in Runner.rc, so there is no
        // extra directory level (this is where 0.1.x installs already keep library.db).
        let app_data = Path::new(r"C:\Users\mix\AppData\Roaming");
        let resolved = support_dir(Host::Windows, None, None, Some(app_data));
        assert_eq!(resolved.parent(), Some(app_data));
        assert_eq!(
            resolved.file_name(),
            Some(std::ffi::OsStr::new(WINDOWS_PRODUCT_NAME))
        );
    }

    #[test]
    fn missing_environment_falls_back_to_the_working_directory() {
        assert_eq!(
            support_dir(Host::Linux, None, None, None),
            Path::new("./top.mixar.app")
        );
    }

    /// The directory name must keep matching the platform metadata, or the data
    /// directory moves and an existing library looks lost.
    #[test]
    fn leaves_match_the_platform_metadata() {
        let linux = include_str!("../../../apps/gui-flutter/linux/CMakeLists.txt");
        assert!(
            linux.contains(&format!("set(APPLICATION_ID \"{APP_ID}\")")),
            "linux APPLICATION_ID no longer matches {APP_ID}"
        );

        let macos = include_str!("../../../apps/gui-flutter/macos/Runner/Configs/AppInfo.xcconfig");
        assert!(
            macos.contains(&format!("PRODUCT_BUNDLE_IDENTIFIER = {APP_ID}")),
            "macOS bundle identifier no longer matches {APP_ID}"
        );

        let rc = include_str!("../../../apps/gui-flutter/windows/runner/Runner.rc");
        assert!(
            rc.contains(&format!(
                "VALUE \"ProductName\", \"{WINDOWS_PRODUCT_NAME}\""
            )),
            "Windows ProductName no longer matches {WINDOWS_PRODUCT_NAME}"
        );
        assert!(
            rc.contains("VALUE \"CompanyName\", \"\""),
            "a non-empty CompanyName adds a directory level under %APPDATA%"
        );
    }
}
