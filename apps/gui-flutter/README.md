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

```sh
cd apps/gui-flutter
mise exec -- flutter_rust_bridge_codegen generate   # after Rust API changes
mise exec -- flutter run -d linux
```

Smoke UI: pick a backend, list devices, start/stop the engine.

## Rust tests

```sh
cargo test --manifest-path crates/Cargo.toml -p host_flutter
```

Uses the `null` backend (no real audio device).
