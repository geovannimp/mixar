# Developer guide

Practical notes for people hacking on Mixar itself. See the [website developer docs](apps/website/src/content/docs/developers) for subsystem references (MIDI mappings, [set history](apps/website/src/content/docs/developers/history/overview.mdx)). Runtime contracts and the library collections model: [`docs/tech-spec.md`](docs/tech-spec.md).

## High-level architecture

```
mixar/
├─ crates/                # Cargo workspace root (Cargo.toml)
│  ├─ audio-core/         # shared traits/types: AudioBackend, AudioSource, StreamParams, DeviceId
│  ├─ backend-null/       # deterministic backend for tests and CI
│  ├─ backend-miniaudio/  # miniaudio implementation
│  ├─ backend-cpal/       # CPAL implementation (native PipeWire on Linux when available)
│  ├─ engine-core/        # engine lifecycle, config, producer thread, track loading
│  │  ├─ lib.rs           # module declarations and public re-exports
│  │  ├─ config.rs        # EngineConfig and related types
│  │  ├─ engine.rs        # Engine public API
│  │  ├─ backend.rs       # backend factory (AudioBackend::list_names / new)
│  │  ├─ producer.rs      # ring buffer, MasterStreamSetup, producer thread loop
│  │  ├─ callback.rs      # ConsumerCallback (ring-buffer consumer)
│  │  └─ audio_source/    # FileAudioSource; re-exports AudioSource / LoadedAudio
│  ├─ engine-dsp/         # pure DSP: deck, mixer channel graph (no I/O)
│  │  ├─ lib.rs           # DspEngine
│  │  ├─ deck.rs          # playback/transport only
│  │  ├─ mixer_lane.rs    # graph node: deck + strip
│  │  ├─ mixer_channel.rs # per-lane strip (gain/EQ/filter/VU/fader)
│  │  └─ mixer.rs         # process lanes, crossfade-sum, bus routing
│  ├─ codec/              # decoder wrapper (symphonia)
│  ├─ resampler/          # resampler trait + rubato impl (pluggable)
│  ├─ library-core/       # Library traits + Collection/Track types
│  ├─ library/            # library manager (canonical writable store)
│  ├─ library-adapters/   # third-party formats (Mixxx, Rekordbox, …)
│  ├─ analyzer-core/      # offline analysis traits and types
│  ├─ analyzer-qmdsp/     # pure-Rust qm-dsp (Mixxx) beat/key backend
│  └─ analyzer/           # decode + analyze_file facade
├─ apps/gui-flutter/      # Flutter desktop UI (FRB host: crates/host-flutter)
└─ samples/               # sample audio for local demos
```

### Data flow

```
AudioSource (e.g. FileAudioSource)
        │ load() → LoadedAudio
        ▼
   Engine::load_track → Deck (dry playback) + MixerChannel (auto gain)
        │  (deck resamples at playback; channel applies gain/EQ/filter/fader)
        ▼
Producer thread ──► ring buffer ──► audio callback (backend)
   (DspEngine::process)              (ConsumerCallback)
```

**Design principles:**

- All crates are small and focused.
- `engine-dsp` is pure Rust and has zero I/O dependencies (no filesystem, network, codec, or backend imports).
- `audio-core` defines the runtime trait boundary that backends implement, plus `AudioSource` / `LoadedAudio`.
- Track loading is pluggable via `AudioSource`. Concrete I/O loaders (e.g. `FileAudioSource`) live outside `engine-dsp`.
- There is no separate `backend-pipewire` crate; use `backend-cpal` for native PipeWire on Linux.

## Application logging

The Flutter desktop host and Rust crates share [`tracing`](https://docs.rs/tracing) on the Rust side. Dart/Flutter UI logging is still lightweight (console / `debugPrint`); there is no unified LogTape-style frontend pipeline yet.

### Stack

| Layer | Library | Role |
| --- | --- | --- |
| Rust crates / `host-flutter` | [`tracing`](https://docs.rs/tracing) | Facade in engine/library/controller crates; macros `tracing::{error,warn,info,debug,trace}!` |
| Host init (`init_app`) | [`tracing-subscriber`](https://docs.rs/tracing-subscriber) + [`tracing-log`](https://docs.rs/tracing-log) | stderr + optional app-support file via tee writer; `EnvFilter` (default `info`, override with `RUST_LOG`); bridges leftover `log` crate calls from dependencies |
| Flutter / Dart | console / `debugPrint` | UI diagnostics during development |

### Where app data lives

Bundle / application id: `top.mixar.app` (Flutter desktop / app-support directory).

Library DB and settings sit next to each other under the platform application-support directory:

| Platform | Directory (typical) |
| --- | --- |
| Linux | `$XDG_DATA_HOME/top.mixar.app` or `~/.local/share/top.mixar.app` |
| macOS | `~/Library/Application Support/top.mixar.app` |
| Windows | `%APPDATA%\top.mixar.app` |

Files of interest: `library.db`, `settings.json`, `mixar.log` (Rust diagnostics; attached when `ControllerTransport` starts).

### Raising verbosity

- **Rust:** set `RUST_LOG` (e.g. `RUST_LOG=debug`, `RUST_LOG=engine_core=debug,controller=info`) when launching the app; prefer temporary `tracing::debug!` in the crate under investigation over inventing a second logging stack.
- **Flutter:** use `debugPrint` / DevTools; avoid noisy production `print` in hot paths.

### Notes

Controller MIDI/Rhai failures are written through `tracing` (and therefore into `mixar.log` once attached). Log rotation / shared Dart categories remain a follow-up — do not reintroduce a Tauri/LogTape pipeline or a parallel facade for first-party Mixar code.

## Codec & resampling

- **Codec:** [`symphonia`](https://crates.io/crates/symphonia) via the `codec` crate (`read_frames`, `load_entire_file`). Do not expose symphonia types in the public API. Consumers are `AudioSource` implementations (e.g. `FileAudioSource`). Never call codec from `engine-dsp`.
- **Resampler:** [`rubato`](https://crates.io/crates/rubato) behind the `resampler` crate’s `Resampler` trait (pluggable for A/B tests). Decks in `engine-dsp` resample source audio to the engine/stream sample rate **during playback**; tracks stay at native rate after `AudioSource::load()`.

## Audio backends

All backends are compiled into the binary (subject to Cargo features) and chosen at runtime via config or `AudioBackend::new(name)`. No dynamic loading.

| Backend | Role |
| --- | --- |
| `backend-cpal` | Default path. Native PipeWire on Linux when available. Optional `backend-cpal` feature on `engine-core` (default-on). Preferred under `backend = "auto"`. |
| `backend-miniaudio` | Cross-platform fallback under `"auto"` when CPAL is unavailable. |
| `backend-null` | Deterministic timing for tests/CI; no real device. |

There is no `backend-pipewire` crate. Platform-native exclusive backends (ASIO / WASAPI exclusive / CoreAudio low-latency): [#453](https://github.com/geovannimp/mixar/issues/453).

## WASM / web

Not implemented. `engine-dsp` stays I/O-free so a future WASM build remains feasible. Plan and approach: [#59](https://github.com/geovannimp/mixar/issues/59).

## Performance targets

Default config: `buffer_size = 512`, `sample_rate = 48000` (buffer duration ≈ 10.67 ms at 48 kHz).

Under a typical Linux x86_64 machine with two decks playing stereo files:

- No audible glitches (no persistent xruns) under normal conditions.
- Callback worst-case execution time under 50% of buffer duration (≈ 5.3 ms at default).
- Underruns write silence; producer warmup + ring-buffer prefill cover startup.

Always measure latency-sensitive paths with `--release`. For distribution, set explicit `target-cpu` or build per-target. Document realtime scheduling for users (`chrt` / `setcap`) when they need real-time priority.

## Release profile tweaks

```toml
[profile.release]
opt-level = 3      # "z" disables loop vectorization
lto = true
codegen-units = 1
panic = "abort"
```

## CI, testing & maintainability

Root `npm` scripts / moon drive affected checks (see [`README.md`](README.md)). Rust gates typically include `cargo fmt -- --check`, `cargo clippy -- -D warnings`, and `cargo test` (prefer `backend = "null"` for headless runs). Optional `cargo bench` / criterion for DSP hot paths ([#245](https://github.com/geovannimp/mixar/issues/245)).

Rules of thumb:

- Keep each crate small with a single responsibility.
- Keep `engine-core` split across modules (`config`, `engine`, `backend`, `producer`, `callback`, …); do not collapse into a single `lib.rs`.
- Stable `audio-core` trait surface; minimize breaking changes.
- `apps/gui-flutter` is the reference desktop host.
