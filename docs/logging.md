# Application logging

The GUI app uses one logging pipeline for Rust (Tauri host) and the React frontend.

## Stack

| Layer | Library | Role |
| --- | --- | --- |
| Rust crates / Tauri host | [`log`](https://docs.rs/log) + [`tauri-plugin-log`](https://v2.tauri.app/plugin/logging/) | Facade in crates; plugin is the subscriber in `apps/gui-app` |
| React / TypeScript | [LogTape](https://logtape.org/) + `@tauri-apps/plugin-log` | Hierarchical categories; under Tauri, a sink forwards into the same plugin |

`env_logger` is not used by the GUI host (it would double-init with the plugin). The headless `app-example` binary may still use `env_logger` + `RUST_LOG`.

## Targets (Rust plugin)

Configured in `apps/gui-app/src-tauri/src/lib.rs`:

- **Stdout** — terminal output for `tauri dev` / CI
- **LogDir** — persisted files under the platform log directory (default file name = application name)

Default max level: **Debug** in debug builds, **Info** in release. Noisy crates (`sqlx`, `sea_orm`, `tracing`) are capped at **Warn**.

## Where log files live

Bundle identifier: `com.geovanni.gui-app` (see `tauri.conf.json`).

| Platform | Directory |
| --- | --- |
| Linux | `$XDG_DATA_HOME/com.geovanni.gui-app/logs` or `~/.local/share/com.geovanni.gui-app/logs` |
| macOS | `~/Library/Logs/com.geovanni.gui-app` |
| Windows | `%LocalAppData%\com.geovanni.gui-app\logs` |

Files use the application name by default (e.g. `gui-app.log`) (plus rotations when size limits apply).

## Raising verbosity

- **Rust / plugin:** rebuild in debug for Debug-level host logs, or temporarily change `.level(...)` / `.level_for(...)` on the plugin builder in `lib.rs`.
- **Headless example:** `RUST_LOG=debug,sqlx=warn cargo run -p app-example` (uses `env_logger`).
- **Frontend (LogTape):** categories under `["app", …]` use Debug in Vite/Tauri **dev**, Info in production builds. Prefer `engineLogger` / `libraryLogger` / `waveformLogger` / `controllerLogger` from `apps/gui-app/src/lib/logging.ts` over raw `console.*`. Pass `Error` values with LogTape’s `(message, error)` overloads (use `asError(unknown)` at catch boundaries).

## Frontend entrypoint

`logging.ts` calls `configureSync()` at import time (LogTape SPA pattern). `main.tsx` imports it first, then `attachTauriLogging()` lazy-loads the Tauri sink so JS logs also reach Stdout/LogDir. DevTools visibility for JS stays on LogTape’s console sink (Rust host logs remain on Stdout/LogDir).
