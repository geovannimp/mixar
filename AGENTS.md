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

- Root `npm install` installs [lefthook](https://lefthook.dev) via npm workspaces (`gui-app` package) and `prepare` → `lefthook install`; pre-commit runs `rustfmt` on staged `*.rs` with `stage_fixed`. Skip job: `LEFTHOOK_EXCLUDE=cargo-fmt`; disable: `LEFTHOOK=0`; emergency: `git commit --no-verify`. Prefer not bypassing the hook.
- Cargo workspace: `audio-core`, `engine-core`, `engine-dsp`, `backend-cpal`, `backend-null`, `backend-miniaudio`, `library*`, `codec`, `resampler`, `analyzer*`, plus `app-example` CLI and `gui-app` (Tauri + React; npm workspace).
- Headless audio engine uses a producer thread writing interleaved stereo into a lock-free ring buffer; the backend audio callback consumes it.
- Default engine config: 48 kHz sample rate, 512-frame buffer size (latency tied to buffer size).
- `engine-core` default features enable `backend-cpal`; CPAL is the primary Linux backend (native PipeWire when available).
- `gui-app` is the Tauri desktop UI over the engine; folder browser supports collections created from folders.
- Device optimization in `backend-cpal` prefers 2-channel config with lowest min sample rate and smallest buffer size when opening streams.
- `docs/deck-spec.md` defines target deck UI, features, data model, and phased roadmap.
- Engine emits events to the UI for state sync with external control sources (e.g. MIDI); GUI subscribes and renders.
- Hot cues and loops persist in dedicated `track_hot_cue` / `track_loop` tables in `library.db` (same pattern as waveforms).
