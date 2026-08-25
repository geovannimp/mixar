# Application logging

The Flutter desktop host and Rust crates share the `log` facade on the Rust side. Dart/Flutter UI logging is still lightweight (console / `debugPrint`); there is no unified LogTape-style frontend pipeline yet.

## Stack

| Layer | Library | Role |
| --- | --- | --- |
| Rust crates / `host-flutter` | [`log`](https://docs.rs/log) | Facade in engine/library/controller crates; host init via FRB defaults (`init_app`) |
| Flutter / Dart | console / `debugPrint` | UI diagnostics during development |

## Where app data lives

Bundle / application id: `top.mixar.app` (Flutter desktop / app-support directory).

Library DB and settings sit next to each other under the platform application-support directory:

| Platform | Directory (typical) |
| --- | --- |
| Linux | `$XDG_DATA_HOME/top.mixar.app` or `~/.local/share/top.mixar.app` |
| macOS | `~/Library/Application Support/top.mixar.app` |
| Windows | `%APPDATA%\top.mixar.app` |

Files of interest: `library.db`, `settings.json`.

## Raising verbosity

- **Rust:** enable a `log` subscriber in the host or run with crate-level filters when debugging; prefer temporary `log::debug!` in the crate under investigation over inventing a second logging stack.
- **Flutter:** use `debugPrint` / DevTools; avoid noisy production `print` in hot paths.

## Notes

A fuller unified logging story (file rotation, shared categories across Rust and Dart) is a follow-up — do not reintroduce a Tauri/LogTape pipeline.
