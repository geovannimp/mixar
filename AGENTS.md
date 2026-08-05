## Learned User Preferences

- Follow `docs/tech-spec.md` for architecture and module boundaries.
- Follow `docs/deck-spec.md` for deck UI and phased features; group performance controls (tempo, loops, hot cues) in bordered panels; disable them when no track is loaded; do not persist deck/waveform layout resizes across sessions.
- Use `dasp` for internal sample/frame types and conversions; use `dasp_graph` for the mixer graph.
- Use `rubato` for resampling.
- Read real device capabilities from CPAL (channels, sample-rate ranges); avoid hardcoded defaults like fixed channel counts or sample rates in device listing.
- In the Tauri GUI, use coss UI components and toast notifications (including `toast.promise` for async startup) instead of verbose inline status messages.
- Custom in-app title bar on Tauri (decorations off), matching app chrome, with min/max/close controls.
- Prefer minimal, focused changes when refining UI; user may narrow scope mid-task.
- Use cpal with low-latency/realtime features as the default audio backend.
- Audio backend API: `AudioBackend::list_names()` and `AudioBackend::new(name)`; enumerate devices on the backend instance; mark default devices with an `is_default` flag on `DeviceInfo`.
- Auto-start the audio engine when entering the decks view; no manual start button.
- Exit the application with an error if the engine fails to load rather than continuing in a broken state.

## Learned Workspace Facts

- Cargo workspace root is `crates/Cargo.toml` (members: `audio-core`, `engine-core`, `engine-dsp`, `backend-cpal`, `backend-null`, `backend-miniaudio`, `library*`, `codec`, `resampler`, `analyzer*`, `app-example`). Run cargo via `cargo --manifest-path crates/Cargo.toml …` (or `cd crates`). Desktop UI is `apps/gui-app` (Tauri + React; npm/moon globs `apps/*` / `packages/*`). rust-analyzer uses `.vscode/settings.json` `linkedProjects` for `crates/Cargo.toml` and `apps/gui-app/src-tauri/Cargo.toml`.
- Root `npm install` installs [lefthook](https://lefthook.dev) and [moon](https://moonrepo.dev) (`@moonrepo/cli`) via the npm workspace root; `prepare` → `lefthook install`. Pre-commit runs `format:staged` / `lint:staged` (moon `format-files` / `lint-files`: rustfmt + crates clippy when `.rs` staged; gui-app oxfmt/oxlint with typeCheck). Skip jobs: `LEFTHOOK_EXCLUDE=lint,format`; disable: `LEFTHOOK=0`; emergency: `git commit --no-verify`. Prefer not bypassing the hook. Toolchain pins: Node 22 via `.node-version` / `package.json` `engines`; Rust via `rust-toolchain.toml` (stable + rustfmt/clippy). [mise](https://mise.jdx.dev) is recommended locally and used in CI. Affected full-package checks use `npx moon ci` / root scripts `lint`, `format:check`, `build`.
- Headless audio engine uses a producer thread writing interleaved stereo into a lock-free ring buffer; the backend audio callback consumes it.
- Default engine config: 48 kHz sample rate, 512-frame buffer size (latency tied to buffer size).
- `engine-core` default features enable `backend-cpal`; CPAL is the primary Linux backend (native PipeWire when available).
- `apps/gui-app` is the Tauri desktop UI over the engine; folder browser supports collections created from folders.
- Device optimization in `backend-cpal` prefers 2-channel config with lowest min sample rate and smallest buffer size when opening streams.
- `docs/deck-spec.md` defines target deck UI, features, data model, and phased roadmap.
- Engine emits events to the UI for state sync with external control sources (e.g. MIDI); GUI subscribes and renders.
- Engine event bus: `engine-core` owns omnibus cmd/evt buses; hosts bridge MessagePack bytes only (`engine_publish` / `engine://bus`); frontend uses `EngineTransport` (not direct Tauri invoke/listen for engine traffic). JSON `engine://event` is retired. See `docs/superpowers/specs/2026-07-26-engine-event-bus-design.md`.
- Library I/O uses `LibraryTransport` (separate from the engine bus). Hosts prepare `PreparedTrackPlayback` outside the `AppState` lock; never hold `AppState` while waiting on `library`.
- Controller / MIDI mapping I/O uses `ControllerTransport` (same rule: no raw Tauri `invoke`/`listen` from React).
- Hot cues and loops persist in dedicated `track_hot_cue` / `track_loop` tables in `library.db` (same pattern as waveforms).
