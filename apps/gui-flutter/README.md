# gui-flutter (experimental)

Flutter desktop host for rust-mixer, bridged to Rust via
[flutter_rust_bridge](https://github.com/fzyzcjy/flutter_rust_bridge).

Design: [`docs/superpowers/specs/2026-08-10-flutter-desktop-host-design.md`](../../docs/superpowers/specs/2026-08-10-flutter-desktop-host-design.md)

## Layout

| Path | Role |
|------|------|
| `apps/gui-flutter` | Flutter app (linux / macOS / windows / web enabled; Linux verified) |
| `crates/host-flutter` | FRB Rust host (`host_flutter`) over `engine-core` + library browse |

Tauri (`apps/gui-app`) stays the primary UI until this experiment replaces it.

Library browse opens `{getApplicationSupportDirectory()}/library.db` (app id `com.geovanni.gui-app`, shared with Tauri; desktop only — web shows a placeholder). UI: Forui `FItemGroup` collections + [trina_grid](https://github.com/doonfrs/trina_grid) tracks ([design](../../docs/superpowers/specs/2026-08-10-flutter-library-browse-design.md)). FRB `LibraryTransport` also exposes add-folder / resolve / track list + `getTrack` (artwork only on `getTrack` until covers live in `library.db`) and a thin typed analyze/refresh event stream ([parity design](../../docs/superpowers/specs/2026-08-13-flutter-library-transport-parity-design.md)); waveform rasterization stays in Flutter. The shell UI does not wire the extra RPCs yet.

Generated FRB outputs under `lib/src/rust/` and `crates/host-flutter/src/frb_generated.rs` are **committed** (FRB’s usual workflow so clones build without running codegen first). Regenerate after Rust API changes.

## Prerequisites

From the repo root (Flutter + ninja are pinned in `mise.toml`):

```sh
mise install   # Flutter postinstall activates dashmonx (https://github.com/rosenpin/dashmonx)
export PATH="$HOME/.cargo/bin:$PATH"   # for flutter_rust_bridge_codegen
cargo install flutter_rust_bridge_codegen --locked   # once, if missing
```

## Develop

From the repo root (`dashmonx` wraps `flutter run` and hot-reloads on `lib/` changes):

```sh
npm run flutter:dev
# or: moon run gui-flutter:dev-linux
```

Desktop uses [`window_manager`](https://pub.dev/packages/window_manager) with `TitleBarStyle.hidden` and in-app min/max/close controls (drag empty header regions to move; double-click to maximize).

After Rust API changes:

```sh
moon run gui-flutter:generate
```

The main window is a Forui ([forui.dev](https://forui.dev/)) layout shell: header, waveforms, decks/mixer, library — engine/library not wired into the shell yet. Display name comes from Rust (`engine_api::APP_DISPLAY_NAME` / `appDisplayName()`).

## Rust tests

```sh
cargo test --manifest-path crates/Cargo.toml -p host_flutter
```

Uses the `null` backend (no real audio device).
