# Flutter Desktop Host (Experimental) — Design

**Date:** 2026-08-10  
**Status:** Approved for implementation (user waived per-section gates)  
**Related:** `docs/tech-spec.md`, `docs/superpowers/specs/2026-07-26-engine-event-bus-design.md`, existing host `apps/gui-app` (Tauri)

## Goal

Add an experimental Flutter desktop host that talks to the Rust engine via [flutter_rust_bridge](https://github.com/fzyzcjy/flutter_rust_bridge). Start greenfield. If the experiment succeeds, Flutter may replace Tauri; until then Tauri stays untouched and primary.

**Milestone (smoke):** Flutter app builds/runs on Linux; lists audio backends and output devices; starts and stops the engine through a thin FRB host crate.

## Non-goals (this milestone)

- Deck / mixer / library / controller UI
- MessagePack omnibus bus parity with Tauri (`engine_publish` / evt stream)
- Extracting shared `host-core` from Tauri
- Verifying macOS, Windows, or Web (targets enabled only)
- Exiting the process on engine start failure (Tauri policy can come later)
- CI job for Flutter (optional follow-up)

## Layout

```text
apps/gui-flutter/          # Flutter app (linux, macos, windows, web enabled)
crates/host-flutter/       # FRB Rust host; member of crates/ workspace
apps/gui-app/              # Tauri — unchanged
```

Scaffold with FRB’s Cargo workspace flow:

```sh
# from apps/
flutter_rust_bridge_codegen create gui_flutter \
  --rust-crate-dir ../../crates/host-flutter \
  --rust-crate-name host_flutter \
  --platforms linux,macos,windows,web
```

Then rename/move the Flutter directory to `apps/gui-flutter` if the generator used underscores, remove any generated `Cargo.lock` under `crates/host-flutter`, and add `host-flutter` to `crates/Cargo.toml` workspace members.

## Tooling

- **Flutter:** install and pin via [mise](https://mise.jdx.dev) in repo `mise.toml` (`flutter = "3.44.9"`, plus `ninja` for Linux desktop builds). No system Flutter, no FVM.
- **FRB codegen:** `flutter_rust_bridge_codegen` via cargo install / binstall (document version used).
- **Rust:** existing workspace toolchain (`rust-toolchain.toml`).
- Run Flutter under mise (`mise exec -- flutter …` or activated shell).

## Architecture

```text
┌─────────────────────┐     FRB (typed)      ┌──────────────────────┐
│  apps/gui-flutter   │ ◄──────────────────► │ crates/host-flutter  │
│  smoke UI (Dart)    │                      │ session + API        │
└─────────────────────┘                      └──────────┬───────────┘
                                                        │
                                                        ▼
                                              ┌─────────────────────┐
                                              │ engine-core         │
                                              │ EngineSession /     │
                                              │ AudioBackend        │
                                              └─────────────────────┘
```

`host-flutter` is a thin typed FRB surface over `engine-core` / `audio-core`. No MessagePack bridge yet. Tauri remains the reference host for full product behavior.

## Host API (FRB)

Process-local session behind a mutex / `OnceLock`.

| Function | Behavior |
|----------|----------|
| `list_backend_names() -> Vec<String>` | `AudioBackend::list_names()` |
| `list_output_devices(backend: String) -> Vec<OutputDevice>` | `create_backend` → `list_output_devices`; map fields |
| `start_engine(backend, sample_rate?, buffer_size?) -> ()` | Build `EngineConfig` (default 48 kHz / 512), `EngineSession::new` + `engine.start()`. **Idempotent:** if already running, return `Ok(())` without restart |
| `stop_engine() -> ()` | Stop engine and drop session |
| `engine_is_running() -> bool` | Whether a session exists |

**`OutputDevice` fields:** `id: String`, `name: String`, `is_default: bool`, `max_channels: u16`, `default_sample_rates: Vec<u32>`.

Errors return as FRB/`anyhow`-style failures to Dart; the Flutter process stays alive. Surface failures in the smoke UI.

Default backend for start when unspecified: `"auto"`.

## Flutter smoke UI

Single screen:

- Backend dropdown (from `list_backend_names`)
- Refresh + list of output devices for selected backend
- Start / Stop engine buttons
- Running status text
- Last error text (if any)

No decks, waveforms, or settings persistence.

## Platforms

- Enable: Linux, macOS, Windows, Web
- Verify this milestone: **Linux desktop only**
- Web/macOS/Windows may fail to run; that is acceptable for now

## Testing / acceptance

1. `mise install` provides `flutter`
2. `flutter_rust_bridge_codegen generate` succeeds for the host
3. `cargo test -p host-flutter` covers list backends / start-stop with `"null"` backend (no real device required)
4. On Linux: `flutter run -d linux` shows backends, devices, and can start/stop engine with a real or auto backend

## Follow-ups (out of scope)

- Omnibus MessagePack bridge (Tauri parity)
- Shared host extraction
- Deck UI / library
- Flutter CI + moon tasks
- Hard-exit on engine failure when Flutter becomes primary
