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
- **LogDir** — persisted files under the platform log directory (file name `gui-app`)
- **Webview** — Rust (and forwarded JS) logs visible in DevTools when the frontend calls `attachConsole()` (dev only)

Default max level: **Debug** in debug builds, **Info** in release. Noisy crates (`sqlx`, `sea_orm`, `tracing`) are capped at **Warn**.

## Where log files live

Bundle identifier: `com.geovanni.gui-app` (see `tauri.conf.json`).

| Platform | Directory |
| --- | --- |
| Linux | `$XDG_DATA_HOME/com.geovanni.gui-app/logs` or `~/.local/share/com.geovanni.gui-app/logs` |
| macOS | `~/Library/Logs/com.geovanni.gui-app` |
| Windows | `%LocalAppData%\com.geovanni.gui-app\logs` |

Files are named like `gui-app.log` (plus rotations when size limits apply).

## Raising verbosity

- **Rust / plugin:** rebuild in debug for Debug-level host logs, or temporarily change `.level(...)` / `.level_for(...)` on the plugin builder in `lib.rs`.
- **Headless example:** `RUST_LOG=debug,sqlx=warn cargo run -p app-example` (uses `env_logger`).
- **Frontend (LogTape):** categories under `["gui", …]` use Debug in Vite/Tauri **dev**, Info in production builds. Prefer `engineLog` / `libraryLog` / `waveformLog` / `controllerLog` from `apps/gui-app/src/lib/logging.ts` over raw `console.*`.

## Frontend entrypoint

`configureAppLogging()` runs from `main.tsx` before React mounts. In Tauri dev it also `attachConsole()` so the Webview target shows in DevTools.
