# Rust DJ Engine

A modular, high-performance Rust audio engine for DJ applications.

## Overview

Headless Rust library providing a reusable audio engine for DJ apps. It features runtime-selectable audio backends, modular decks, pluggable audio loading via `AudioSource`, and a producer/consumer threading model with lock-free ring buffers.

## Architecture

Cargo workspace layout:

```
rust-dj-engine/
├─ audio-core/         # Shared types and traits (AudioBackend, AudioSource, Sample, …)
├─ backend-null/       # Deterministic backend for tests and CI
├─ backend-miniaudio/  # Miniaudio backend
├─ backend-cpal/       # CPAL backend (native PipeWire on Linux when available)
├─ engine-core/        # Engine lifecycle, config, producer thread, track loading
├─ engine-dsp/         # Pure DSP: decks, mixer, analyzers (no I/O)
├─ codec/              # Decoder wrapper (symphonia)
├─ resampler/          # Resampler trait + rubato implementation
├─ library/            # Tag reader + metadata manager (placeholder)
├─ app-example/        # Minimal example binary
└─ samples/            # Sample audio for local demos
```

### Data flow

```
AudioSource (e.g. FileAudioSource)
        │ load() → LoadedAudio
        ▼
   Engine::load_track → Deck (engine-dsp)
        │
        ▼
Producer thread ──► ring buffer ──► audio callback (backend)
   (DSP process)                    (ConsumerCallback)
```

- **Producer:** engine-controlled thread runs DSP and writes interleaved stereo into a preallocated ring buffer.
- **Consumer:** backend audio callback reads from the ring buffer; no allocations on the audio thread.
- **Backends:** chosen at runtime via config (`auto` / `cpal` / `miniaudio` / `null`). Compiled in; no dynamic loading.

### engine-core modules

| Module | Responsibility |
|--------|----------------|
| `config` | `EngineConfig` and related TOML types |
| `engine` | `Engine` public API (`start` / `stop` / `load_track` / `play` / `pause`) |
| `backend` | Backend factory (`AudioBackend::list_names` / `new`) |
| `producer` | Ring buffer setup and producer thread loop |
| `callback` | `ConsumerCallback` (ring-buffer consumer for the audio device) |
| `audio_source` | `FileAudioSource`; re-exports `AudioSource` / `LoadedAudio` from `audio-core` |

### Audio loading

Tracks are loaded through the `AudioSource` trait (defined in `audio-core`), not a bare path:

```rust
use engine_core::{Engine, EngineConfig, FileAudioSource};

let mut engine = Engine::new(EngineConfig::default())?;
engine.start()?;
engine.load_track(0, &FileAudioSource::new("track.wav"))?;
engine.play(0)?;
```

- `AudioSource` / `LoadedAudio` live in `audio-core` so loaders stay independent of engine internals.
- `FileAudioSource` (in `engine-core`) loads from disk via `codec`.
- Implement `AudioSource` for other origins (memory, network, etc.) without changing `Engine`.

### engine-dsp

Pure DSP only (`deck`, `mixer`, `analyzer`). No filesystem, network, or backend/codec I/O. Suitable for future WASM builds.

## Current Status

Working pieces:

- Workspace crates and CI skeleton
- `audio-core` traits (`AudioBackend`, `AudioStream`, `AudioCallback`, `AudioSource`)
- Backends: `null`, `miniaudio`, `cpal`
- `codec` (symphonia) and `resampler` (rubato)
- Producer/consumer plumbing with ring buffer
- `Engine` API with `AudioSource`-based `load_track`
- `app-example` demo (config, backend discovery, load/play/pause)

Still open / partial:

- Full bus/device channel mapping (`set_bus_device` stub)
- Library manager and tag storage (Sprint 3 placeholder)
- WASM build of `engine-dsp`

## Quick Start

### Prerequisites

- Rust 1.70+ (stable, beta, or nightly)
- Linux x86_64 (primary development platform)
- For real audio output: a working sound device (CPAL/miniaudio). Use `backend = "null"` for headless tests.

### Building

```bash
git clone <repository-url>
cd rust-dj-engine

cargo build
cargo test
cargo run -p app-example
```

The example loads a file from `samples/` when present. Override backend and settings with a local `config.toml` or by editing `app-example`.

### Running Tests

```bash
cargo test
cargo test -p engine-core --lib
cargo test -p audio-core
cargo test -p backend-null
cargo test -p engine-dsp
```

Integration tests that open real devices may fail without audio hardware; prefer the null backend for CI-style runs.

## Development

### Code Style

- `cargo fmt`
- `cargo clippy`
- Standard Rust naming conventions

### Testing

- Unit tests live in each crate
- Integration tests can use the null backend
- CI runs format, clippy, and tests on Linux

### CI/CD

GitHub Actions includes format checking, Clippy, tests, security auditing, and docs generation.

## Roadmap

### Done

- [x] Workspace scaffolding
- [x] `audio-core` traits and types
- [x] `backend-null`, `backend-miniaudio`, `backend-cpal`
- [x] `codec` (symphonia) and `resampler` (rubato)
- [x] Producer/consumer ring-buffer plumbing
- [x] Pluggable `AudioSource` loading (`FileAudioSource`)

### Next

- [ ] Complete bus/device channel mapping
- [ ] Library manager: collections, tag reading, and metadata storage
- [ ] WASM prototype for `engine-dsp`

## License

GPL-3.0

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests
5. Run `cargo fmt` and `cargo clippy`
6. Submit a pull request

## Technical Details

See [docs/tech-spec.md](docs/tech-spec.md) for the full technical specification. Cursor rules under `.cursor/rules/` describe per-crate conventions for agents and contributors.
