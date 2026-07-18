# Rust DJ Engine

A modular, high-performance Rust audio engine for DJ applications.

## Overview

Headless Rust library providing a reusable audio engine for DJ apps. It features runtime-selectable audio backends, modular decks, pluggable audio loading via `AudioSource`, and a producer/consumer threading model with lock-free ring buffers.

## Architecture

Cargo + npm workspace layout:

```
rust-dj-engine/
├─ package.json        # npm workspaces root (gui-app + lefthook + @moonrepo/cli)
├─ .moon/              # moon workspace + toolchains
├─ moon.yml            # rust (Cargo workspace) moon project
├─ lefthook.yml        # pre-commit rustfmt + oxfmt/oxlint (staged files)
├─ audio-core/         # Shared types and traits (AudioBackend, AudioSource, Sample, …)
├─ backend-null/       # Deterministic backend for tests and CI
├─ backend-miniaudio/  # Miniaudio backend
├─ backend-cpal/       # CPAL backend (native PipeWire on Linux when available)
├─ engine-core/        # Engine lifecycle, config, producer thread, track loading
├─ engine-dsp/         # Pure DSP: decks, mixer (no I/O)
├─ codec/              # Decoder wrapper (symphonia)
├─ resampler/          # Resampler trait + rubato implementation
├─ library/            # Library manager (collections, tags, analysis)
├─ library-core/       # Library traits and shared types
├─ analyzer-core/      # Offline analysis traits and types
├─ analyzer-stratum/   # stratum-dsp backend
├─ analyzer/           # decode + analyze_file facade
├─ app-example/        # Minimal example binary
├─ gui-app/            # Tauri + React desktop UI (npm workspace package; moon.yml)
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

Pure DSP only (`deck`, `mixer`). No filesystem, network, or backend/codec I/O. BPM/key/beat grid come from library track metadata. Suitable for future WASM builds.

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

### Git hooks

Run `npm install` once at the **repo root**. That installs [lefthook](https://lefthook.dev) and [moon](https://moonrepo.dev) (`@moonrepo/cli`), wires pre-commit hooks, and enables the task graph.

Pre-commit still runs `rustfmt` / `oxfmt` / `oxlint --fix` on **staged** files only (`stage_fixed`). CI uses `moon ci` for affected full-package checks.

- Skip a lefthook job: `LEFTHOOK_EXCLUDE=cargo-fmt` / `oxfmt` / `oxlint`
- Disable lefthook: `LEFTHOOK=0 git commit ...`
- Emergency only: `git commit --no-verify`

### moon task runner

```bash
npm install                 # root — hooks + gui-app + moon
npm run lint                # moon run :lint
npm run format:check        # moon run :format-check
npm run build               # moon run :build
npm run gui:dev             # moon run gui-app:dev
npm run gui:build           # moon run gui-app:build
npm run gui:tauri           # moon run gui-app:tauri (pass args after --)
npx moon ci --base main     # locally mimic affected CI
```

#### Adding a new npm workspace package (e.g. `website`, `docs`)

1. Add the directory to root `package.json` `workspaces`.
2. Add `package.json` with `lint`, `format:check`, and `build` scripts.
3. Add `moon.yml` (`language: typescript`) whose tasks call those scripts; set `runInCI: false` on `dev`.
4. Register the project in `.moon/workspace.yml` if not covered by a glob.
5. `npm install` at root; verify `npx moon run <id>:lint` and that `npx moon ci --base main` only runs it when that package changes.

### Code Style

- `cargo fmt` (also enforced by the pre-commit hook)
- `cargo clippy`
- Standard Rust naming conventions

### Testing

- Unit tests live in each crate
- Integration tests can use the null backend
- CI runs format, clippy, and tests on Linux

### CI/CD

GitHub Actions primary gate is `moon ci` (affected lint, format-check, build, and rust test). A secondary Rust beta/nightly matrix runs on Rust path changes. Separate jobs cover security audit and docs generation.

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
5. Run `npm install` at the repo root (installs git hooks); then `cargo fmt` / `cargo clippy` as needed
6. Submit a pull request

## Technical Details

See [docs/tech-spec.md](docs/tech-spec.md) for the full technical specification. Cursor rules under `.cursor/rules/` describe per-crate conventions for agents and contributors.
