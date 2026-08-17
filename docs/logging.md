# Application logging

The GUI app uses one logging pipeline for Rust (Tauri host) and the React frontend.

## Stack

| Layer | Library | Role |
| --- | --- | --- |
| Rust crates / Tauri host | [`log`](https://docs.rs/log) + [`tauri-plugin-log`](https://v2.tauri.app/plugin/logging/) | Facade in crates; plugin is the subscriber in `apps/gui-app` |
| React / TypeScript | [LogTape](https://logtape.org/) + `@tauri-apps/plugin-log` | Hierarchical categories; under Tauri, a sink forwards into the same plugin |

`env_logger` is not used by the GUI host (it would double-init with the plugin).

## Targets (Rust plugin)

Configured in `apps/gui-app/src-tauri/src/lib.rs`:

- **Stdout** — terminal output for `tauri dev` / CI
- **LogDir** — persisted files under the platform log directory (default file name = application name)

Default max level: **Debug** in debug builds, **Info** in release. Noisy crates (`sqlx`, `sea_orm`, `tracing`) are capped at **Warn**.

## Where log files live

Bundle identifier: `top.mixar.app` (see `tauri.conf.json`).

| Platform | Directory |
| --- | --- |
| Linux | `$XDG_DATA_HOME/top.mixar.app/logs` or `~/.local/share/top.mixar.app/logs` |
| macOS | `~/Library/Logs/top.mixar.app` |
| Windows | `%LocalAppData%\top.mixar.app\logs` |

Files use the application name by default (e.g. `Mixar.log`) (plus rotations when size limits apply).

## Raising verbosity

- **Rust / plugin:** rebuild in debug for Debug-level host logs, or temporarily change `.level(...)` / `.level_for(...)` on the plugin builder in `lib.rs`.
- **Frontend (LogTape):** categories under `["app", …]` use Debug in Vite/Tauri **dev**, Info in production builds. Prefer `engineLogger` / `libraryLogger` / `waveformLogger` / `controllerLogger` from `apps/gui-app/src/lib/logging.ts` over raw `console.*`. Pass `Error` values with LogTape’s `(message, error)` overloads (use `asError(unknown)` at catch boundaries).

## Frontend entrypoint

`logging.ts` configures LogTape at import time (SPA pattern; `configure()` under Tauri for the async plugin sink, `configureSync()` in the browser). `main.tsx` imports it first. When `APP_ENVIRONMENT === "TAURI"`, it lazy-imports `@/lib/tauri-sink` so JS logs also reach Stdout/LogDir; DevTools stays on the console sink (Rust host logs remain on Stdout/LogDir).
