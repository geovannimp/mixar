## Learned User Preferences

- Follow `docs/tech-spec.md` for architecture and module boundaries.
- Use `dasp` for internal sample/frame types and conversions; use `dasp_graph` for the mixer graph.
- Use `rubato` for resampling.
- Read real device capabilities from CPAL (channels, sample-rate ranges); avoid hardcoded defaults like fixed channel counts or sample rates in device listing.
- In the Tauri GUI, use coss UI components and toast notifications instead of verbose inline status messages.
- Custom in-app title bar on Tauri (decorations off), matching app chrome, with min/max/close controls.
- Prefer minimal, focused changes when refining UI; user may narrow scope mid-task.
- Use cpal with low-latency/realtime features as the default audio backend.
- Audio backend API: `AudioBackend::list_names()` and `AudioBackend::new(name)`; enumerate devices on the backend instance; mark default devices with an `is_default` flag on `DeviceInfo`.
- Exit the application with an error if the engine fails to load rather than continuing in a broken state.

## Learned Workspace Facts

- Cargo workspace: `audio-core`, `engine-core`, `engine-dsp`, `backend-cpal`, `backend-null`, `backend-miniaudio`, `library*`, `codec`, `resampler`, `analyzer*`, plus `app-example` CLI and `gui-app` (Tauri + React).
- Headless audio engine uses a producer thread writing interleaved stereo into a lock-free ring buffer; the backend audio callback consumes it.
- Default engine config: 48 kHz sample rate, 512-frame buffer size (latency tied to buffer size).
- `engine-core` default features enable `backend-cpal`; CPAL is the primary Linux backend (native PipeWire when available).
- `gui-app` is the Tauri desktop UI over the engine; folder browser supports collections created from folders.
- Device optimization in `backend-cpal` prefers 2-channel config with lowest min sample rate and smallest buffer size when opening streams.
