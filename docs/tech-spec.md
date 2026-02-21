# Tech Spec — Modular Rust Audio Engine (Backend-Only)

## Table of Contents

- [Tech Spec — Modular Rust Audio Engine (Backend-Only)](#tech-spec--modular-rust-audio-engine-backend-only)
  - [Table of Contents](#table-of-contents)
  - [1 — Project Overview](#1--project-overview)
  - [1 — Objectives (What MVP Must Deliver)](#1--objectives-what-mvp-must-deliver)
    - [Core Requirements](#core-requirements)
  - [2 — Non-goals for MVP](#2--non-goals-for-mvp)
  - [3 — High-level Architecture](#3--high-level-architecture)
  - [4 — Core Runtime Contracts \& APIs](#4--core-runtime-contracts--apis)
    - [4.1 audio-core (Types \& Traits)](#41-audio-core-types--traits)
    - [4.2 Engine Entry Points (engine-core)](#42-engine-entry-points-engine-core)
  - [5 — Threading \& Buffer Model](#5--threading--buffer-model)
    - [5.1 Two-thread Design](#51-two-thread-design)
      - [Producer Thread (engine-controlled)](#producer-thread-engine-controlled)
      - [Consumer (audio callback)](#consumer-audio-callback)
    - [5.2 Ring Buffer \& Zero Allocations](#52-ring-buffer--zero-allocations)
    - [5.3 Channel Mapping](#53-channel-mapping)
  - [6 — Config Schema (TOML in User Config Directory)](#6--config-schema-toml-in-user-config-directory)
    - [Example Configuration](#example-configuration)
    - [Configuration Notes](#configuration-notes)
  - [7 — Codec \& Resampling Choices](#7--codec--resampling-choices)
    - [Codec (Decoding)](#codec-decoding)
    - [Resampler](#resampler)
  - [8 — Backends](#8--backends)
    - [8.1 backend-miniaudio](#81-backend-miniaudio)
    - [8.2 backend-pipewire (Linux)](#82-backend-pipewire-linux)
    - [8.3 backend-null](#83-backend-null)
    - [Runtime Selection](#runtime-selection)
  - [9 — WASM / Web Support](#9--wasm--web-support)
    - [Approach](#approach)
  - [10 — Metadata / Library Manager \& Tag Handling](#10--metadata--library-manager--tag-handling)
    - [MVP Requirements](#mvp-requirements)
    - [Design Decision](#design-decision)
  - [11 — Acceptance Criteria \& Performance](#11--acceptance-criteria--performance)
    - [Performance Target (MVP)](#performance-target-mvp)
    - [Functional Acceptance](#functional-acceptance)
  - [12 — CI, Testing \& Maintainability](#12--ci-testing--maintainability)
    - [CI](#ci)
    - [Tests](#tests)
    - [Maintainability Rules](#maintainability-rules)
  - [13 — Build Flags \& Runtime Tweaks (Recommended)](#13--build-flags--runtime-tweaks-recommended)
    - [Notes](#notes)
  - [14 — Roadmap (Phases)](#14--roadmap-phases)
    - [MVP (Deliverable)](#mvp-deliverable)
    - [v1](#v1)
    - [v2](#v2)
  - [15 — Implementation Plan \& Milestone Tasks (Developer-friendly)](#15--implementation-plan--milestone-tasks-developer-friendly)
    - [Sprint 0 — Workspace \& Scaffolding](#sprint-0--workspace--scaffolding)
    - [Sprint 1 — Miniaudio \& Codec](#sprint-1--miniaudio--codec)
    - [Sprint 2 — Producer/Consumer Plumbing](#sprint-2--producerconsumer-plumbing)
    - [Sprint 3 — Library Manager \& Tags](#sprint-3--library-manager--tags)
    - [Sprint 4 — PipeWire \& WASM Prototyping](#sprint-4--pipewire--wasm-prototyping)
  - [16 — Example Config \& Usage (Quick)](#16--example-config--usage-quick)
    - [Configuration File](#configuration-file)
    - [Minimal App Usage (Pseudo)](#minimal-app-usage-pseudo)
  - [17 — Acceptance \& Next Steps (What I Will Deliver Next If You Want)](#17--acceptance--next-steps-what-i-will-deliver-next-if-you-want)

---

## 1 — Project Overview

- **Project name (placeholder):** `rust-dj-engine`
- **License:** GPLv3
- **Primary dev / CI platform:** Linux x86_64
- **Delivery form:** Rust crate (workspace) + optional static library build

## 1 — Objectives (What MVP Must Deliver)

Headless Rust library (crate) providing a reusable audio engine for DJ apps.

### Core Requirements

- **Runtime-selectable audio backend** (miniaudio primary on all platforms; PipeWire supported on Linux). No compile-time feature lock for selection — backends compiled in and chosen at runtime.

- **Two modular decks** (playback units) in the MVP; decks are modular so more can be added later.

- **Per-deck/cue/master audio routing** with per-bus device + channel mapping (stereo pairs only).

- **Configurable buffer size** (default 512) and sample rate (default 48 kHz but overridable in config). Latency is determined by buffer size.

- **Two-thread model:** engine-controlled producer thread writes decoded/resampled audio into a ring buffer; audio callback thread (consumer) reads from it.

- **Support common audio formats** via a robust decoder (e.g., symphonia), high-quality but fast resampling via rubato (pluggable).

- **Tag reading** (read metadata from files) and ability to store per-track metadata (hotcues, BPM tag) — see storage notes below.

- **WASM support** (compile DSP to WASM) with multiple outputs in the browser (using miniaudio where feasible or WASM+WebAudio glue).

- **Strong maintainability:** small crates, tests, null backend, CI build on Linux x86_64, static analysis.

## 2 — Non-goals for MVP

- Time-stretching / professional pitch-shift (plan modular extension later).
- Mixer GUI or UI (library is headless).
- Telemetry / opt-in data collection.
- Recording/streaming.
- Android/iOS packaging or Raspberry Pi-specific packaging for MVP (architect supports them later).
- WASAPI exclusive/ASIO native low-latency backends (target for v2).

## 3 — High-level Architecture

```
rust-dj-engine/ (Cargo workspace)
├─ engine-core/        # orchestrates engine lifecycle, clock, scheduler, config
├─ engine-dsp/         # pure DSP: deck, mixer graph (minimal), analyzers
├─ audio-core/         # shared traits/types: AudioBackend, StreamParams, DeviceId
├─ backend-miniaudio/  # miniaudio implementation
├─ backend-pipewire/   # pipewire implementation (Linux)
├─ backend-null/       # Null backend for testing
├─ codec/              # decoder wrapper (symphonia)
├─ resampler/          # resampler trait + rubato impl (pluggable)
├─ library/            # tag reader + metadata manager (headless)
└─ app-example/        # minimal example binary wiring engine + backend selection
```

**Design Principles:**

- All crates are small and focused
- `engine-dsp` is pure Rust and has zero I/O dependencies
- `audio-core` defines the runtime trait boundary that backends implement

## 4 — Core Runtime Contracts & APIs

### 4.1 audio-core (Types & Traits)

Key types (Rust pseudo):

```rust
// audio-core/src/lib.rs

use std::time::Duration;

pub type Sample = f32; // internal sample format

#[derive(Clone, Debug)]
pub struct DeviceId(pub String);

#[derive(Clone, Debug)]
pub struct DeviceInfo {
    pub id: DeviceId,
    pub name: String,
    pub max_channels: u16,
    pub default_sample_rates: Vec<u32>,
}

#[derive(Clone, Debug)]
pub struct StreamParams {
    pub sample_rate: u32,
    pub channels: u16,                 // e.g., 2 for stereo
    pub frames_per_buffer: u32,        // requested frames (e.g., 512)
    pub low_latency: bool,
}

pub trait AudioCallback: Send {
    /// Fill `out` with interleaved samples: out.len() == frames * channels
    fn render(&mut self, out: &mut [Sample], frames: u32, sr: u32);
}

pub trait AudioStream: Send {
    fn start(&mut self) -> anyhow::Result<()>;
    fn stop(&mut self) -> anyhow::Result<()>;
    fn actual_buffer_size(&self) -> Option<u32>; // what backend granted
    fn actual_latency(&self) -> Option<Duration>;
}

pub trait AudioBackend: Send + Sync {
    fn name(&self) -> &'static str;
    fn list_output_devices(&self) -> anyhow::Result<Vec<DeviceInfo>>; // each DeviceInfo has is_default
    fn open_output_stream(
        &mut self,
        device: &DeviceId,
        params: &StreamParams,
        callback: Box<dyn AudioCallback>,
    ) -> anyhow::Result<Box<dyn AudioStream>>;
}
```

**Note:** `AudioCallback::render` is the consumer (audio thread) function that the backend will call.

### 4.2 Engine Entry Points (engine-core)

The engine owns the producer thread and implements ring-buffer writing plus decode/resample pipeline. Public surface (simplified):

```rust
pub struct EngineConfig { /* see config schema below */ }
pub struct Engine { /* holds backend, dsp, producer thread handles */ }

impl Engine {
    pub fn new(config: EngineConfig) -> anyhow::Result<Self>;
    pub fn start(&mut self) -> anyhow::Result<()>;
    pub fn stop(&mut self) -> anyhow::Result<()>;
    pub fn load_track(&mut self, deck_id: usize, path: &str) -> anyhow::Result<()>;
    pub fn play(&mut self, deck_id: usize) -> anyhow::Result<()>;
    pub fn pause(&mut self, deck_id: usize) -> anyhow::Result<()>;
    pub fn set_bus_device(&mut self, bus: BusId, device: DeviceId, channels: [u16;2]) -> anyhow::Result<()>;
    // plus control API to set sample rate, buffer size etc.
}
```

Engine will spawn a producer thread that decodes / decimates / resamples audio into per-bus ring buffers.

## 5 — Threading & Buffer Model

### 5.1 Two-thread Design

#### Producer Thread (engine-controlled)

- Decodes audio (via codec crate), resamples (via resampler crate) to the engine internal sample rate, applies pre-mixer transforms (gain/track-level).
- Writes interleaved stereo frames to a lock-free ring buffer for each output bus.
- Wakes on commands (play/stop/seek/hotcue) from control thread via control channel (lock-free mailbox).

#### Consumer (audio callback)

- Implemented by the backend (miniaudio/pipewire). Backend's real-time callback reads from ring buffer into its output buffer, applying final mixing if needed (mixing can be done in producer or consumer depending on latency profile; recommended: producer does mixing into bus-specific ring buffers, consumer just copies to device output).
- The backend calls `AudioCallback::render()` (which pulls the requested frames from ring buffer(s) and writes into the backend's buffer).

### 5.2 Ring Buffer & Zero Allocations

- Use a tested lock-free ring buffer crate (or an in-tree ring buffer, `heapless::spsc::Queue` or `concurrentqueue` ported safe).
- Preallocate buffer to hold at least `N * frames_per_buffer` where `N = e.g., 8` (configurable) to tolerate producer jitter.
- **No heap allocations in the audio callback.** All buffers preallocated. No mutexes in audio callback.

### 5.3 Channel Mapping

Bus is always a stereo pair (2 channels). Each Bus maps to a specific device and two contiguous device channels or arbitrary selected device channels (config supports explicit indexes).

**Example:** DDJ-400 mapping where master uses channels 3–4 and cue uses 1–2.

## 6 — Config Schema (TOML in User Config Directory)

**Default config path:** `$XDG_CONFIG_HOME/rust-dj-engine/config.toml` (fallback `~/.config/rust-dj-engine/config.toml`)

### Example Configuration

```toml
[engine]
sample_rate = 48000         # engine internal SR, default 48k
buffer_size = 512           # frames per buffer
low_latency = false         # hint for backend
backend = "auto"            # "auto", "miniaudio", or "pipewire"

# device definitions
[[device]]
name = "Focusrite USB"
id = "hw:1,0"               # human-friendly ID (backend maps it)

# bus routing: bus -> device + channel pair
[[bus]]
name = "master"
device = "default"          # device id or "default"
channels = [3, 4]           # device channels (1-based indexing recommended)

[[bus]]
name = "cue"
device = "default"
channels = [1, 2]
```

### Configuration Notes

- Engine interprets channels as 1-based channel indexes and maps to the device's available channels.
- `backend = "auto"` tries to detect PipeWire when present; otherwise selects miniaudio.

## 7 — Codec & Resampling Choices

### Codec (Decoding)

Use **symphonia** (Rust) for decoding common formats (MP3, AAC, FLAC, WAV, Ogg Vorbis). It's well-maintained and supports many formats. Wrap it in codec crate that presents a streaming API: `Decoder::read_frames(&mut [f32]) -> frames`.

### Resampler

**Default resampler:** rubato (high quality, FFT-based), wrapped inside the resampler crate exposing trait `Resampler`:

```rust
pub trait Resampler: Send {
    fn process(&mut self, in_buf: &[f32], out_buf: &mut [f32], channels: usize) -> usize;
    fn set_rate(&mut self, input_sr: u32, output_sr: u32);
}
```

Make implementation pluggable so you can swap `speedxdsp` or others later for A/B tests.

## 8 — Backends

### 8.1 backend-miniaudio

Primary cross-platform backend. Uses miniaudio via Rust bindings or via FFI wrapper crate.

- Implements `AudioBackend` trait.
- On Linux, miniaudio will enumerate ALSA/PulseAudio/Jack/PipeWire where available — but to support PipeWire directly we also provide backend-pipewire.

### 8.2 backend-pipewire (Linux)

Uses pipewire-rs bindings for low-latency native PipeWire streams and provides precise buffer/latency control.

- Implement channel mapping and device selection.
- If PipeWire is not present, `make_backend("pipewire")` returns an informative error and auto fallback uses miniaudio.

### 8.3 backend-null

Provides a deterministic backend for tests and CI. Simulates audio timing and supports buffer size negotiation.

### Runtime Selection

All backends are compiled into the binary crate. The app chooses backend at runtime using the config. No dynamic loading.

## 9 — WASM / Web Support

**Goal:** compile DSP and engine control logic to WASM and connect to the browser's WebAudio for output with multiple outputs (cue/master).

### Approach

Build `engine-dsp` and resampler to WASM with `wasm-pack` / `wasm-bindgen`. Provide a thin JS glue that:

1. Installs an `AudioWorkletProcessor` that pulls audio from WASM (shared ring buffer or MessagePort).
2. Creates two `MediaStreamAudioDestinationNodes` (master & cue). For multiple outputs: duplicate the audio stream into two destinations and use `setSinkId()` on `HTMLAudioElements` when supported. If the browser lacks `setSinkId()`, fallback to single output or show limitation to the host app (but you said you don't care about implementation as long as it works — so the library will provide a working approach leveraging miniaudio WASM if feasible).

The WASM engine will still follow the 2-thread model logically (producer thread simulated via Web Worker) and use `SharedArrayBuffer` / `Atomics` or message passing for controls.

**Note:** Browser limitations mean you may need to adapt how multiple outputs work; provide best-effort and document limitations.

## 10 — Metadata / Library Manager & Tag Handling

### MVP Requirements

- Read tags from audio files (ID3, Vorbis comments, FLAC metadata) using symphonia or a dedicated tag reading crate (e.g., lofty).
- Store per-track operational metadata (hotcues, beatgrid, corrected BPM) as tags where possible AND mirror them into an internal library database for reliability and performance.

### Design Decision

**Primary read:** use file tags for initial metadata (BPM, cues) when present.

**Persistent storage for app-managed metadata:** keep an internal SQLite DB (crate: library) that stores library entries and custom fields (hotcues, beatgrid corrections, user BPM). For portability, support writing back to file tags as a secondary operation (optional; may be format-dependent) — but the canonical store for runtime data is the internal DB. This keeps write operations safe, and the tracks metadata portable across file formats.

**Note:** You previously asked to "store all data as tags"; keeping an internal DB + optional write-back keeps the UX safe while allowing tag portability later.

## 11 — Acceptance Criteria & Performance

### Performance Target (MVP)

**Default config:** `buffer_size = 512`, `sample_rate = 48000`.

Under typical dev machine (Linux x86_64, modern CPU), with two decks playing stereo files and one cue output:

- **No audible glitches** (no persistent xruns) under normal conditions.
- **Callback worst-case execution time** must be < 50% of buffer duration:
  - Buffer duration at 48k/512 ≈ 10.67 ms → worst-case callback < 5.3 ms.
- If underruns occur during startup, library should attempt fallback: try larger buffer (double) and return status to caller.

### Functional Acceptance

- Engine loads audio files, reads tags, exposes metadata.
- Engine can be configured via TOML; starts with chosen backend; lists devices and channels.
- Engine allows mapping buses to devices+channel pairs.
- Engine spawns producer thread; audio callback pulls from ring buffer; playback is stable with no dynamic memory allocations in callback.
- API allows embedding the engine as a crate; can also be built as a static library (`cargo build --release --target <...>` producing `libxxx.a`) with a C API wrapper added as a later task.

## 12 — CI, Testing & Maintainability

### CI

Linux x86_64 build + test job (MVP).

**Steps:**

- `cargo fmt -- --check`
- `cargo clippy -- -D warnings`
- `cargo test --release` (unit tests)
- `cargo bench` (optional) for performance regression

Run unit tests for DSP blocks and integration test using backend-null.

### Tests

- Unit tests for DSP modules with golden data.
- NullBackend integration tests to simulate timing, xruns, buffer negotiation.
- Fuzz tests for file parsing and tag reading.
- Benchmarks (criterion) for resampler, mixer, and decode pipeline.

### Maintainability Rules

- Keep each crate small with a single responsibility.
- Stable audio-core trait; minimize breaking changes. Use capability discovery for optional features.
- Use semantic versioning, changelog.
- Document per-backend caveats and supported devices.
- Add examples folder with app-example showing runtime selection and config usage.

## 13 — Build Flags & Runtime Tweaks (Recommended)

Use `Cargo.toml` / `.cargo/config.toml` release profile tuned for audio performance:

```toml
[profile.release]
opt-level = "z"   # or 3 for maximum speed; consider "z" if distribution size matters
lto = true
codegen-units = 1
panic = "abort"

[build]
rustflags = ["-C", "target-cpu=native"]
```

### Notes

- For distribution, set explicit target-cpu or build per-target.
- Always test with `--release`.
- Document realtime scheduling instructions for users (`chrt` or `setcap`) so they can grant real-time priority when needed.

## 14 — Roadmap (Phases)

### MVP (Deliverable)

- Implement workspace skeleton.
- Implement audio-core trait + engine-core and engine-dsp minimal decks.
- Implement backend-miniaudio and backend-null.
- Implement codec (symphonia) and resampler (rubato).
- Implement library reading tags + internal SQLite DB.
- Implement TOML config & runtime selection, device listing, channel mapping, and example app.
- CI on Linux x86_64 with tests.

### v1

- Add backend-pipewire implementation.
- Fine tune latency reporting, buffer negotiation logging (requested vs granted).
- Add safe optional tag write-back support.
- Add WASM build path + JS glue for WebAudio multiple outputs (experimentally).

### v2

- Add optional ASIO/WASAPI/CoreAudio backends for low-latency pro targets (platform-specific builds).
- Add advanced time-stretch/pitch-shift module and plugin hosting hooks (LV2/VST) research.
- Add UI examples and packaging (AppImage/Flatpak/Homebrew).

## 15 — Implementation Plan & Milestone Tasks (Developer-friendly)

### Sprint 0 — Workspace & Scaffolding

- Create workspace, crates, CI skeleton.
- Implement audio-core trait & backend-null.
- Add engine-dsp minimal deck & mixer stub, unit tests.

### Sprint 1 — Miniaudio & Codec

- Implement backend-miniaudio (blocking streaming version).
- Implement codec wrapper (symphonia) + unit tests for decoding files.
- Implement resampler abstraction and rubato impl.
- Add TOML config parsing and example app-example.

### Sprint 2 — Producer/Consumer Plumbing

- Implement ring buffer + producer thread (engine-controlled).
- Integrate producer -> resampler -> ring buffer -> consumer render.
- Add device + channel mapping logic and API.

### Sprint 3 — Library Manager & Tags

- Add library crate: read tags via lofty or symphonia metadata; implement SQLite schema; import tracks.
- Provide APIs for reading/writing metadata; expose hotcue/beatgrid storage in DB. (Write-back optional.)

### Sprint 4 — PipeWire & WASM Prototyping

- Implement backend-pipewire and test on Linux.
- Prototype WASM build for engine-dsp and experiment with WebAudio glue (AudioWorklet).

## 16 — Example Config & Usage (Quick)

### Configuration File

`~/.config/rust-dj-engine/config.toml`:

```toml
[engine]
backend = "auto"
sample_rate = 48000
buffer_size = 512
low_latency = false

[[bus]]
name = "cue"
device = "default"
channels = [1,2]

[[bus]]
name = "master"
device = "default"
channels = [3,4]
```

### Minimal App Usage (Pseudo)

```rust
let cfg = EngineConfig::from_toml_file(config_path)?;
let mut engine = Engine::new(cfg)?;
engine.start()?; // spawns producer & opens backend stream
engine.load_track(0, "trackA.mp3")?;
engine.play(0)?;
```

## 17 — Acceptance & Next Steps (What I Will Deliver Next If You Want)

I can produce the following artifacts as follow-ups (pick any/all):

1. **A GitHub/Cargo workspace skeleton** with audio-core, backend-null, engine-core, engine-dsp, app-example and CI pipeline.

2. **A detailed audio-core trait file** and a working backend-null implementation and tests.

3. **A backend-miniaudio stub** or fully working miniaudio backend (platform testing required).

4. **WASM prototype** showing multi-output in the browser.

---

_This tech spec provides a comprehensive roadmap for building a modular, high-performance Rust audio engine suitable for DJ applications. The architecture emphasizes maintainability, performance, and extensibility while delivering a solid MVP foundation._
