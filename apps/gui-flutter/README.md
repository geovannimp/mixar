# gui-flutter

Flutter desktop host for Mixar, bridged to Rust via
[flutter_rust_bridge](https://github.com/fzyzcjy/flutter_rust_bridge).

This is the **primary** desktop UI.

## Layout

| Path | Role |
|------|------|
| `apps/gui-flutter` | Flutter app (linux / macOS / windows / web enabled; Linux verified) |
| `crates/host-flutter` | FRB Rust host (`host_flutter`) over `engine-core` + library browse |

Library browse opens `{getApplicationSupportDirectory()}/library.db` (app id `top.mixar.app`; desktop only — web shows a placeholder). UI: MixarTheme collections/drive tabs (Shad-bridged), add-folder, analyze/refresh, visible-row artwork, and a sortable track list (shared with the history entry list through one `TrackListView`, so selection, keyboard load, drag-to-deck and the row menu behave identically). In-flight work renders as a per-row overlay: one right-aligned phase pill per job lane (analysis and stem separation run independently, so a track being analysed and split at once shows both) plus a progress bar per job, rather than in the title cell — so progress ticks rebuild one row instead of the whole list. The 3-dot menu stays open during a job; each action disables itself while its own job runs. FRB `LibraryTransport` also exposes add-folder / resolve / track list + `getTrack` (artwork only on `getTrack` until covers live in `library.db`) and a thin typed analyze/refresh event stream; waveform rasterization stays in Flutter.

Generated FRB outputs under `lib/src/rust/` and `crates/host-flutter/src/frb_generated.rs` are **committed** (FRB’s usual workflow so clones build without running codegen first). Regenerate after Rust API changes.

## Prerequisites

From the repo root (Flutter + ninja are pinned in `mise.toml`):

```sh
mise install   # Flutter postinstall activates dashmonx (https://github.com/rosenpin/dashmonx)
export PATH="$HOME/.cargo/bin:$PATH"   # for flutter_rust_bridge_codegen
cargo install flutter_rust_bridge_codegen --version 2.13.0 --locked   # once, if missing / wrong version
```

## Develop

From the repo root (`dashmonx` wraps `flutter run` and hot-reloads on `lib/` changes):

```sh
npm run flutter:dev
# or: moon run gui-flutter:dev-linux
```

Desktop uses [`window_manager`](https://pub.dev/packages/window_manager) with `TitleBarStyle.hidden` and in-app min/max/close controls (drag empty header regions to move; double-click to maximize).

After Rust API changes (requires `flutter_rust_bridge_codegen` 2.13.0):

```sh
moon run gui-flutter:generate
```

The main window wraps **MixarTheme** + **ShadTheme** (intentional dual bridge from `main.dart`): header, waveforms, decks/mixer, library. Prefer Mixar shell widgets and `context.theme` tokens; toast errors with `showMixarToast`. Engine, library, and MIDI controller transports are wired over FRB. Display name comes from Rust (`engine_api::APP_DISPLAY_NAME` / `appDisplayName()`). Typography: Outfit / Space Grotesk / Noto Sans Mono via `MixarFonts` (`lib/shell/app_typography.dart`).

## Rust tests

```sh
cargo nextest run --manifest-path crates/Cargo.toml -p host_flutter
```

Uses the `null` backend (no real audio device).
