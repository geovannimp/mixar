# Application logging

The Flutter desktop host and Rust crates share [`tracing`](https://docs.rs/tracing) on the Rust side. Dart/Flutter UI logging is still lightweight (console / `debugPrint`); there is no unified LogTape-style frontend pipeline yet.

## Stack

| Layer | Library | Role |
| --- | --- | --- |
| Rust crates / `host-flutter` | [`tracing`](https://docs.rs/tracing) | Facade in engine/library/controller crates; macros `tracing::{error,warn,info,debug,trace}!` |
| Host init (`init_app`) | [`tracing-subscriber`](https://docs.rs/tracing-subscriber) + [`tracing-log`](https://docs.rs/tracing-log) | stderr + optional app-support file via tee writer; `EnvFilter` (default `info`, override with `RUST_LOG`); bridges leftover `log` crate calls from dependencies |
| Flutter / Dart | console / `debugPrint` | UI diagnostics during development |

## Where app data lives

Bundle / application id: `top.mixar.app` (Flutter desktop / app-support directory).

Library DB and settings sit next to each other under the platform application-support directory:

| Platform | Directory (typical) |
| --- | --- |
| Linux | `$XDG_DATA_HOME/top.mixar.app` or `~/.local/share/top.mixar.app` |
| macOS | `~/Library/Application Support/top.mixar.app` |
| Windows | `%APPDATA%\top.mixar.app` |

Files of interest: `library.db`, `settings.json`, `mixar.log` (Rust diagnostics; attached when `ControllerTransport` starts).

## Raising verbosity

- **Rust:** set `RUST_LOG` (e.g. `RUST_LOG=debug`, `RUST_LOG=engine_core=debug,controller=info`) when launching the app; prefer temporary `tracing::debug!` in the crate under investigation over inventing a second logging stack.
- **Flutter:** use `debugPrint` / DevTools; avoid noisy production `print` in hot paths.

## Notes

Controller MIDI/Rhai failures are written through `tracing` (and therefore into `mixar.log` once attached). Log rotation / shared Dart categories remain a follow-up — do not reintroduce a Tauri/LogTape pipeline or a parallel facade for first-party Mixar code.
