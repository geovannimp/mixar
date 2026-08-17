<p align="center">
  <img src="docs/mixar-banner.jpg" alt="Mixar" width="100%">
</p>

# Mixar

[![Build](https://github.com/geovannimp/mixar/actions/workflows/build.yml/badge.svg)](https://github.com/geovannimp/mixar/actions/workflows/build.yml)
[![License: GPL-3.0](https://img.shields.io/badge/license-GPL--3.0-blue.svg)](https://github.com/geovannimp/mixar)

Open-source DJ software: dual decks, a mixer, and a library on a Rust audio engine.

Linux is the primary development platform. The app is under active development and is not a released product yet.

## Features

- Dual decks with overview + scrolling waveforms, beat grid, play/pause, seek, and tempo
- Mixer: gain, 3-band EQ, filter, volume, crossfader, cue/PFL, and VU meters
- Hot cues, loops, performance pads, sampler, and beat sync
- Library with folder collections, track metadata, and offline analysis (BPM, key, loudness, artwork)
- MIDI controller mappings
- Runtime-selectable audio backends: CPAL (PipeWire on Linux when available), miniaudio, or `null` for tests

Two desktop hosts share the same engine: **Tauri + React** (`apps/gui-app`, primary) and an experimental **Flutter** host (`apps/gui-flutter`).

## Quick start

**Prerequisites:** [Node](https://nodejs.org/), Rust stable with rustfmt/clippy, and a working sound device. On Linux, install [Tauri’s native deps](https://v2.tauri.app/start/prerequisites/#linux) plus ALSA/PipeWire headers (`pkg-config`, `libasound2-dev`, `libpipewire-0.3-dev`, `clang` on Debian/Ubuntu).

[mise](https://mise.jdx.dev) is recommended: it reads `.node-version` and `rust-toolchain.toml`.

```bash
git clone https://github.com/geovannimp/mixar.git
cd mixar
mise install
npm install
npm run tauri:dev
```

Sample tracks live in `samples/`. Headless tests can use `backend = "null"` so they do not need an audio device.

The Flutter host (Linux): `npm run flutter:dev`.

## Development

`npm install` at the repo root installs [lefthook](https://lefthook.dev) and [moon](https://moonrepo.dev). Pre-commit runs format and lint on staged files.

```bash
npm run lint            # moon run :lint
npm run format:check    # moon run :format-check
npm run test            # moon run :test
npm run build           # moon run :build
npx moon ci --base main # mimic affected CI locally
```

Skip a hook job with `LEFTHOOK_EXCLUDE=lint,format`. Disable hooks with `LEFTHOOK=0`. Emergency only: `git commit --no-verify`.

CI (GitHub Actions) runs affected lint, format, build, tests, and a cargo audit. A secondary Rust beta/nightly job runs when Rust paths change.

## Architecture

A producer thread runs DSP and writes interleaved stereo into a lock-free ring buffer. The backend audio callback only consumes that buffer — no allocations on the audio thread.

```text
Library / AudioSource
        │ load → PCM
        ▼
   Engine → decks + mixer (engine-dsp)
        │
        ▼
Producer thread ──► ring buffer ──► audio callback (CPAL / miniaudio / null)
```

Hosts talk to the engine and library over MessagePack buses (`engine-api`, `library-api`), not by calling `Engine` from the UI thread.

```text
mixar/
├─ apps/gui-app/       # Tauri + React desktop UI
├─ apps/gui-flutter/   # Experimental Flutter host
├─ crates/             # Cargo workspace (engine, backends, library, controller, …)
└─ samples/            # Local demo audio
```

Default engine config is 48 kHz, 512-frame buffers. Backends are compiled in and chosen at runtime (`auto` / `cpal` / `miniaudio` / `null`).

To embed the engine without a GUI:

```rust
use engine_core::{AudioSource, Engine, EngineConfig, FileAudioSource};

let mut engine = Engine::new(EngineConfig::default())?;
engine.start()?;
engine.load_track(0, AudioSource::File(FileAudioSource::from_path("track.wav")))?;
engine.play(0)?;
```

## Documentation

- [Technical spec](docs/tech-spec.md) — crates, threading, config, backends
- [Deck spec](docs/deck-spec.md) — deck UI, mixer, pads, data model
- [Logging](docs/logging.md) — log files and verbosity
- [Waveforms](docs/dj-waveform-spec.md) and [analyzer](docs/audio-analyzer-spec.md)

## Contributing

Open a pull request against `main`. Run `npm install` once so git hooks are installed, then keep `cargo fmt` / `cargo clippy` and the npm format/lint scripts clean.

## License

[GPL-3.0](https://www.gnu.org/licenses/gpl-3.0.html)
