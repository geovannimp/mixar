# gui-flutter (experimental)

Flutter desktop host for rust-mixer, bridged to Rust via
[flutter_rust_bridge](https://github.com/fzyzcjy/flutter_rust_bridge).

Design: [`docs/superpowers/specs/2026-08-10-flutter-desktop-host-design.md`](../../docs/superpowers/specs/2026-08-10-flutter-desktop-host-design.md)

## Layout

| Path | Role |
|------|------|
| `apps/gui-flutter` | Flutter app (linux / macos / windows / web enabled) |
| `crates/host-flutter` | FRB Rust host (`host_flutter`) over `engine-core` |

Tauri (`apps/gui-app`) stays the primary UI until this experiment replaces it.

## Prerequisites

From the repo root (Flutter + ninja are pinned in `mise.toml`):

```sh
mise install
export PATH="$HOME/.cargo/bin:$PATH"   # for flutter_rust_bridge_codegen
cargo install flutter_rust_bridge_codegen --locked   # once, if missing
```

## Develop

From the repo root:

```sh
npm run flutter:dev
# or: moon run gui-flutter:dev
```

Desktop uses [`window_manager`](https://pub.dev/packages/window_manager) with `TitleBarStyle.hidden` and in-app min/max/close controls (drag empty header regions to move; double-click to maximize).

After Rust API changes:

```sh
moon run gui-flutter:generate
```

Smoke UI: pick a backend, list devices, start/stop the engine.

The main window is a Forui ([forui.dev](https://forui.dev/)) layout shell: header, waveforms, decks/mixer, library — no engine wiring yet.

## Rust tests

```sh
cargo test --manifest-path crates/Cargo.toml -p host_flutter
```

Uses the `null` backend (no real audio device).
