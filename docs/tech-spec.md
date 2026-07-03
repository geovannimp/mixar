# Tech Spec — Modular Rust Audio Engine (Backend-Only)

## Table of Contents

- [1 — Project Overview](#1--project-overview)
- [1 — Objectives (What MVP Must Deliver)](#1--objectives-what-mvp-must-deliver)
- [2 — Non-goals for MVP](#2--non-goals-for-mvp)
- [3 — High-level Architecture](#3--high-level-architecture)
- [4 — Core Runtime Contracts & APIs](#4--core-runtime-contracts--apis)
  - [4.1 audio-core (Types & Traits)](#41-audio-core-types--traits)
  - [4.2 Engine Entry Points (engine-core)](#42-engine-entry-points-engine-core)
  - [4.3 Audio loading (`AudioSource`)](#43-audio-loading-audiosource)
- [5 — Threading & Buffer Model](#5--threading--buffer-model)
- [6 — Config Schema](#6--config-schema)
- [7 — Codec & Resampling Choices](#7--codec--resampling-choices)
- [8 — Backends](#8--backends)
- [9 — WASM / Web Support](#9--wasm--web-support)
- [10 — Metadata / Library Manager & Tag Handling](#10--metadata--library-manager--tag-handling)
- [11 — Acceptance Criteria & Performance](#11--acceptance-criteria--performance)
- [12 — CI, Testing & Maintainability](#12--ci-testing--maintainability)
- [13 — Build Flags & Runtime Tweaks](#13--build-flags--runtime-tweaks)
- [14 — Roadmap (Phases)](#14--roadmap-phases)
- [15 — Implementation Plan & Milestone Tasks](#15--implementation-plan--milestone-tasks)
- [16 — Example Config & Usage](#16--example-config--usage)

---

## 1 — Project Overview

- **Project name:** `rust-dj-engine`
- **License:** GPLv3
- **Primary dev / CI platform:** Linux x86_64
- **Delivery form:** Rust crate (workspace) + optional static library build

## 1 — Objectives (What MVP Must Deliver)

Headless Rust library (crate) providing a reusable audio engine for DJ apps.

### Core Requirements

- **Runtime-selectable audio backend** (`auto` / `cpal` / `miniaudio` / `null`). Backends are compiled in and chosen at runtime (no dynamic loading). On Linux, CPAL provides native PipeWire when available; there is no separate `backend-pipewire` crate.

- **Two modular decks** (playback units) in the MVP; decks are modular so more can be added later.

- **Per-deck/cue/master audio routing** with per-bus device + channel mapping (stereo pairs only). Mapping API exists; full multi-bus device routing is still incomplete.

- **Configurable buffer size** (default 512) and sample rate (default 48 kHz, overridable in config). Latency is determined by buffer size.

- **Two-thread model:** engine-controlled producer thread runs DSP and writes interleaved stereo into a lock-free ring buffer; the backend audio callback (consumer) reads from it.

- **Pluggable track loading** via `AudioSource` (trait in `audio-core`). Disk files use `FileAudioSource` (in `engine-core`), which decodes through the `codec` crate (symphonia).

- **Common audio formats** via symphonia; high-quality resampling via rubato (pluggable), applied in-deck at playback to the engine/stream sample rate.

- **Tag reading** and per-track metadata storage (hotcues, BPM) — library crate (placeholder; Sprint 3).

- **WASM support** (compile DSP to WASM) with multiple outputs in the browser — future work.

- **Strong maintainability:** small crates, tests, null backend, CI on Linux x86_64, static analysis.

## 2 — Non-goals for MVP

- Time-stretching / professional pitch-shift (plan modular extension later).
- Mixer GUI or UI (library is headless).
- Telemetry / opt-in data collection.
- Recording/streaming.
- Android/iOS packaging or Raspberry Pi-specific packaging for MVP (architecture supports them later).
- WASAPI exclusive/ASIO native low-latency backends (target for v2).
- A dedicated `backend-pipewire` crate (use `backend-cpal` instead).

## 3 — High-level Architecture

```
rust-dj-engine/ (Cargo workspace)
├─ audio-core/         # shared traits/types: AudioBackend, AudioSource, StreamParams, DeviceId
├─ backend-null/       # deterministic backend for tests and CI
├─ backend-miniaudio/  # miniaudio implementation
├─ backend-cpal/       # CPAL implementation (native PipeWire on Linux when available)
├─ engine-core/        # engine lifecycle, config, producer thread, track loading
│  ├─ lib.rs           # module declarations and public re-exports
│  ├─ config.rs        # EngineConfig and related types
│  ├─ engine.rs        # Engine public API
│  ├─ backend.rs       # backend factory (AudioBackend::list_names / new)
│  ├─ producer.rs      # ring buffer, MasterStreamSetup, producer thread loop
│  ├─ callback.rs      # ConsumerCallback (ring-buffer consumer)
│  └─ audio_source/    # FileAudioSource; re-exports AudioSource / LoadedAudio
├─ engine-dsp/         # pure DSP: deck, mixer, analyzers (no I/O)
│  ├─ lib.rs           # DspEngine
│  ├─ deck.rs
│  ├─ mixer.rs
│  └─ analyzer.rs
├─ codec/              # decoder wrapper (symphonia)
├─ resampler/          # resampler trait + rubato impl (pluggable)
├─ library/            # tag reader + metadata manager (placeholder)
├─ app-example/        # minimal example binary
└─ samples/            # sample audio for local demos
```

### Data flow

```
AudioSource (e.g. FileAudioSource)
        │ load() → LoadedAudio
        ▼
   Engine::load_track → Deck (engine-dsp)
        │  (samples at native rate; deck resamples at playback)
        ▼
Producer thread ──► ring buffer ──► audio callback (backend)
   (DspEngine::process)              (ConsumerCallback)
```

**Design Principles:**

- All crates are small and focused.
- `engine-dsp` is pure Rust and has zero I/O dependencies (no filesystem, network, codec, or backend imports).
- `audio-core` defines the runtime trait boundary that backends implement, plus `AudioSource` / `LoadedAudio`.
- Track loading is pluggable via `AudioSource`. Concrete I/O loaders (e.g. `FileAudioSource`) live in `engine-core` (or other crates), not in `audio-core` or `engine-dsp`.
- There is no separate `backend-pipewire` crate; use `backend-cpal` for native PipeWire on Linux.

## 4 — Core Runtime Contracts & APIs

### 4.1 audio-core (Types & Traits)

Key types (simplified):

```rust
// audio-core

pub type Sample = f32; // internal sample format

/// Decoded audio ready to load into a deck.
pub struct LoadedAudio {
    pub samples: Vec<Sample>,
    pub sample_rate: u32,
    pub channels: u16,
    /// Identifier for the source (path, URL, etc.).
    pub source_id: String,
}

/// Pluggable audio loader (disk, memory, network, …).
pub trait AudioSource {
    fn load(&self) -> anyhow::Result<LoadedAudio>;
}

// LoadedAudio implements AudioSource (identity load) for already-decoded buffers.

pub struct DeviceId(/* … */);

pub struct DeviceInfo {
    pub id: DeviceId,
    pub name: String,
    pub max_channels: u16,
    pub default_sample_rates: Vec<u32>,
    pub is_default: bool,
}

pub struct StreamParams {
    pub sample_rate: u32,
    pub channels: u16,                 // e.g., 2 for stereo
    pub frames_per_buffer: u32,        // requested frames (e.g., 512)
    pub low_latency: bool,
}

pub trait AudioCallback: Send {
    /// Fill `out` with interleaved samples: out.len() == frames * channels.
    /// Runs on the audio thread: no heap allocations, no mutexes, no blocking.
    fn render(&mut self, out: &mut [Sample], frames: u32, sr: u32);
}

pub trait AudioStream: Send {
    fn start(&mut self) -> anyhow::Result<()>;
    fn stop(&mut self) -> anyhow::Result<()>;
    fn actual_buffer_size(&self) -> Option<u32>;
    fn actual_latency(&self) -> Option<Duration>;
    fn callback_frames_atomic(&self) -> Option<Arc<AtomicU32>>;
}

pub trait AudioBackend: Send + Sync {
    fn name(&self) -> &'static str;
    fn list_output_devices(&self) -> anyhow::Result<Vec<DeviceInfo>>;
    fn open_output_stream(
        &mut self,
        device: &DeviceId,
        params: &StreamParams,
        callback: Box<dyn AudioCallback>,
    ) -> anyhow::Result<Box<dyn AudioStream>>;
}
```

**Note:** `AudioCallback::render` is the consumer (audio thread) function that the backend calls. In `engine-core`, `ConsumerCallback` implements this by popping samples from the ring buffer.

### 4.2 Engine Entry Points (engine-core)

The engine owns the producer thread, opens the backend stream, and loads tracks into decks. Public surface (simplified):

```rust
pub struct EngineConfig { /* see config schema below */ }
pub struct Engine { /* holds backend, dsp, producer thread handles */ }

impl Engine {
    pub fn new(config: EngineConfig) -> anyhow::Result<Self>;
    pub fn start(&mut self) -> anyhow::Result<()>;
    pub fn stop(&mut self) -> anyhow::Result<()>;
    pub fn load_track(&mut self, deck_id: usize, source: &impl AudioSource) -> anyhow::Result<()>;
    pub fn play(&mut self, deck_id: usize) -> anyhow::Result<()>;
    pub fn pause(&mut self, deck_id: usize) -> anyhow::Result<()>;
    pub fn list_devices(&self) -> anyhow::Result<Vec<DeviceInfo>>;
    pub fn default_device(&self) -> anyhow::Result<DeviceInfo>;
    pub fn set_bus_device(&mut self, bus: BusId, device: DeviceId, channels: [u16; 2]) -> anyhow::Result<()>;
    // bus/device config getters/setters
}

/// Factory for listing and creating backends without an engine.
pub struct AudioBackend;
impl AudioBackend {
    pub fn list_names() -> Vec<String>; // e.g. ["null", "miniaudio", "cpal"]
    pub fn new(name: &str) -> anyhow::Result<Box<dyn audio_core::AudioBackend>>;
}
```

`load_track` requires a running engine (`start` first). It calls `source.load()`, then installs samples on the deck at the source’s native sample rate. Decks resample to the engine/stream rate during playback.

### 4.3 Audio loading (`AudioSource`)

| Type | Crate | Role |
|------|--------|------|
| `AudioSource` | `audio-core` | Trait: `load() -> LoadedAudio` |
| `LoadedAudio` | `audio-core` | Decoded interleaved samples + metadata |
| `FileAudioSource` | `engine-core` | Loads from disk via `codec::AudioDecoder` |

Callers never pass a bare path to `Engine::load_track`. Example:

```rust
engine.load_track(0, &FileAudioSource::new("track.wav"))?;
```

New origins (HTTP, in-memory bytes, etc.) implement `AudioSource` without changing `Engine` or `engine-dsp`.

## 5 — Threading & Buffer Model

### 5.1 Two-thread Design

#### Load path (control thread)

- App calls `Engine::load_track(deck_id, &source)`.
- `AudioSource::load()` decodes (e.g. `FileAudioSource` → `codec`) into `LoadedAudio`.
- Engine installs samples on the deck via `Deck::load_audio_samples` (native rate).

#### Producer Thread (engine-controlled)

- Runs `DspEngine::process` for each chunk (decks render/resample, mixer routes).
- Writes interleaved stereo frames to a lock-free ring buffer for the master bus.
- Paced by the device callback count (does not run unbounded ahead of the consumer).

#### Consumer (audio callback)

- Implemented by the backend (`cpal` / `miniaudio` / `null`). The backend’s real-time callback invokes `AudioCallback::render`.
- `ConsumerCallback` pops samples from the ring buffer into the device output buffer. On underrun it writes silence.
- **No heap allocations, no mutexes, no blocking** in the callback path.

### 5.2 Ring Buffer & Zero Allocations

- Use a lock-free ring buffer (`rtrb`).
- Preallocate capacity of at least `N * frames_per_buffer * channels` (current multiplier is higher than the minimum `N ≥ 8`) to tolerate producer jitter.
- Prefill with silence before the stream starts so the callback has data immediately.
- **No heap allocations in the audio callback.** All buffers preallocated.

### 5.3 Channel Mapping

Bus is always a stereo pair (2 channels). Each bus maps to a specific device and two device channels (config supports explicit indexes). Full multi-bus device routing is partially implemented (`set_bus_device` is currently a stub).

**Example:** DDJ-400 mapping where master uses channels 3–4 and cue uses 1–2.

## 6 — Config Schema

Config is loaded via `EngineConfig::from_toml_file` / `to_toml_file`. Fields map directly to the `EngineConfig` struct (flat TOML keys, not a nested `[engine]` table unless you wrap it yourself).

**Conventional paths:** local `config.toml` (as used by `app-example`), or `$XDG_CONFIG_HOME/rust-dj-engine/config.toml` (fallback `~/.config/rust-dj-engine/config.toml`).

### Example Configuration

```toml
sample_rate = 48000
buffer_size = 512
low_latency = false
backend = "auto"            # "auto", "cpal", "miniaudio", or "null"

[[devices]]
name = "Focusrite USB"
id = "hw:1,0"

[[buses]]
# BusConfig fields: id, name, device, channels (see audio-core)
```

### Configuration Notes

- `backend = "auto"` tries CPAL first (when compiled in), then miniaudio, then falls back to null.
- Valid backend names: `"auto"`, `"cpal"`, `"miniaudio"`, `"null"`.
- `backend-cpal` is an optional Cargo feature on `engine-core` (enabled by default).

## 7 — Codec & Resampling Choices

### Codec (Decoding)

Use **symphonia** (Rust) for decoding common formats (MP3, AAC, FLAC, WAV, Ogg Vorbis). Wrap it in the `codec` crate with a streaming API (`read_frames`, `load_entire_file`). Do not expose symphonia types in the public API.

**Consumers:** `AudioSource` implementations such as `FileAudioSource`. Do **not** call codec from `engine-dsp`.

### Resampler

**Default resampler:** rubato (high quality, FFT-based), wrapped inside the `resampler` crate exposing a `Resampler` trait:

```rust
pub trait Resampler: Send {
    fn process(&mut self, in_buf: &[f32], out_buf: &mut [f32], channels: usize) -> usize;
    fn set_rate(&mut self, input_sr: u32, output_sr: u32);
}
```

Make the implementation pluggable so alternatives can be swapped later for A/B tests.

**Engine use:** Decks in `engine-dsp` resample source audio to the engine/stream sample rate **during playback**. Tracks are stored at native rate after `AudioSource::load()`.

## 8 — Backends

### 8.1 backend-miniaudio

Cross-platform backend via miniaudio.

- Implements `AudioBackend`.
- On Linux, may enumerate ALSA/PulseAudio/Jack/PipeWire where available.
- Selected as `"miniaudio"`, or as fallback under `"auto"` when CPAL is unavailable.

### 8.2 backend-cpal

Cross-platform backend via CPAL.

- Implements `AudioBackend`.
- On Linux, uses CPAL’s native PipeWire host when available (this is the supported path for PipeWire; there is no `backend-pipewire` crate).
- Selected as `"cpal"`, and preferred under `"auto"` when initialization succeeds.
- Optional Cargo feature `backend-cpal` on `engine-core` (default-on).

### 8.3 backend-null

Deterministic backend for tests and CI. Simulates audio timing and buffer size negotiation. No real audio device.

- Selected as `"null"`.

### Runtime Selection

All backends are compiled into the binary (subject to features). The app chooses the backend at runtime using config or `AudioBackend::new(name)`. No dynamic loading.

## 9 — WASM / Web Support

**Goal:** compile DSP and engine control logic to WASM and connect to the browser’s WebAudio for output with multiple outputs (cue/master).

### Approach

Build `engine-dsp` and `resampler` to WASM with `wasm-pack` / `wasm-bindgen`. Provide a thin JS glue that:

1. Installs an `AudioWorkletProcessor` that pulls audio from WASM (shared ring buffer or MessagePort).
2. Creates destinations for master & cue outputs, using `setSinkId()` when supported.

The WASM engine should still follow the two-thread model logically (producer simulated via Web Worker) and use `SharedArrayBuffer` / `Atomics` or message passing for controls.

**Status:** Not implemented yet. `engine-dsp` is designed to remain I/O-free so WASM compilation stays feasible.

**Note:** Browser limitations may require adapting how multiple outputs work; provide best-effort and document limitations.

## 10 — Metadata / Library Manager & Tag Handling

### MVP Requirements

- Read tags from audio files (ID3, Vorbis comments, FLAC metadata) using symphonia or a dedicated tag reading crate (e.g., lofty).
- Store per-track operational metadata (hotcues, beatgrid, corrected BPM) and mirror them into an internal library database for reliability and performance.

### Design Decision

**Primary read:** use file tags for initial metadata (BPM, cues) when present.

**Persistent storage for app-managed metadata:** keep an internal SQLite DB (crate: `library`) that stores library entries and custom fields (hotcues, beatgrid corrections, user BPM). Optional write-back to file tags for portability (format-dependent); the canonical store for runtime data is the internal DB.

**Playback:** the library crate does not own the audio device path. Track playback still goes through `engine-core` via `AudioSource` (e.g. `FileAudioSource`).

**Status:** Placeholder crate only; implementation is Sprint 3 work.

## 11 — Acceptance Criteria & Performance

### Performance Target (MVP)

**Default config:** `buffer_size = 512`, `sample_rate = 48000`.

Under a typical dev machine (Linux x86_64, modern CPU), with two decks playing stereo files:

- **No audible glitches** (no persistent xruns) under normal conditions.
- **Callback worst-case execution time** must be < 50% of buffer duration:
  - Buffer duration at 48k/512 ≈ 10.67 ms → worst-case callback < 5.3 ms.
- If underruns occur during startup, the library should attempt fallback (e.g. larger buffer) and return status to the caller (partially addressed via producer warmup and ring-buffer prefill).

### Functional Acceptance

- Engine loads audio via `AudioSource` (e.g. `FileAudioSource`), not a bare path.
- Engine can be configured via TOML; starts with chosen backend; lists devices (`list_devices` / `default_device`).
- Engine allows mapping buses to devices+channel pairs (API present; full routing still incomplete).
- Engine spawns producer thread; audio callback pulls from ring buffer; playback is stable with no dynamic memory allocations in the callback.
- API allows embedding the engine as a crate; can also be built as a static library with a C API wrapper as a later task.

## 12 — CI, Testing & Maintainability

### CI

Linux x86_64 build + test job.

**Steps:**

- `cargo fmt -- --check`
- `cargo clippy -- -D warnings`
- `cargo test` (unit tests; prefer null backend for headless runs)
- `cargo bench` (optional) for performance regression

### Tests

- Unit tests for DSP modules.
- NullBackend integration tests for producer/consumer plumbing and engine control flow.
- Unit tests for `FileAudioSource` (e.g. missing file).
- Fuzz tests for file parsing and tag reading (when library is implemented).
- Benchmarks (criterion) for resampler, mixer, and decode pipeline.

### Maintainability Rules

- Keep each crate small with a single responsibility.
- Keep `engine-core` split across modules (`config`, `engine`, `backend`, `producer`, `callback`, `audio_source`); do not collapse into a single `lib.rs`.
- Stable `audio-core` trait surface; minimize breaking changes.
- Use semantic versioning and a changelog.
- Document per-backend caveats and supported devices.
- `app-example` is the reference for minimal app usage (config, backend discovery, `FileAudioSource`, play/pause).

## 13 — Build Flags & Runtime Tweaks

Use `Cargo.toml` / `.cargo/config.toml` release profile tuned for audio performance:

```toml
[profile.release]
opt-level = "z"   # or 3 for maximum speed; consider "z" if distribution size matters
lto = true
codegen-units = 1
panic = "abort"
```

### Notes

- For distribution, set explicit `target-cpu` or build per-target.
- Always test with `--release` for latency-sensitive checks.
- Document realtime scheduling instructions for users (`chrt` or `setcap`) so they can grant real-time priority when needed.

## 14 — Roadmap (Phases)

### Done (current codebase)

- Workspace skeleton and CI.
- `audio-core` traits (`AudioBackend`, `AudioStream`, `AudioCallback`, `AudioSource`).
- `backend-null`, `backend-miniaudio`, `backend-cpal`.
- `codec` (symphonia) and `resampler` (rubato).
- Producer/consumer ring-buffer plumbing (`rtrb`, `ConsumerCallback`).
- `Engine` API with `AudioSource`-based `load_track` and `FileAudioSource`.
- TOML config (`EngineConfig`) and `app-example`.

### Next (MVP remaining)

- Complete bus/device channel mapping (`set_bus_device` and multi-bus routing).
- Library crate: tag reading + internal SQLite DB.
- Latency reporting / buffer negotiation polish.

### v1

- Optional tag write-back support.
- WASM build path + JS glue for WebAudio multiple outputs (experimental).

### v2

- Optional ASIO/WASAPI/CoreAudio backends for low-latency pro targets (platform-specific builds).
- Advanced time-stretch/pitch-shift module and plugin hosting hooks (LV2/VST) research.
- UI examples and packaging (AppImage/Flatpak/Homebrew).

## 15 — Implementation Plan & Milestone Tasks

### Sprint 0 — Workspace & Scaffolding ✅

- Create workspace, crates, CI skeleton.
- Implement `audio-core` traits & `backend-null`.
- Add `engine-dsp` minimal deck & mixer stub, unit tests.

### Sprint 1 — Backends & Codec ✅

- Implement `backend-miniaudio` and `backend-cpal`.
- Implement `codec` wrapper (symphonia) + unit tests.
- Implement resampler abstraction and rubato impl.
- Add TOML config parsing and `app-example`.

### Sprint 2 — Producer/Consumer & AudioSource ✅ (partial)

- Implement ring buffer + producer thread (engine-controlled).
- Integrate producer → DSP → ring buffer → consumer render.
- Pluggable `AudioSource` loading (`FileAudioSource`).
- Device + channel mapping logic and API (stub / incomplete).

### Sprint 3 — Library Manager & Tags (open)

- Add library crate: read tags via lofty or symphonia metadata; implement SQLite schema; import tracks.
- Provide APIs for reading/writing metadata; expose hotcue/beatgrid storage in DB. (Write-back optional.)

### Sprint 4 — WASM Prototyping (open)

- Prototype WASM build for `engine-dsp` and experiment with WebAudio glue (AudioWorklet).
- PipeWire on Linux is covered by `backend-cpal` (no separate pipewire backend sprint).

## 16 — Example Config & Usage

### Configuration File

Example `config.toml`:

```toml
sample_rate = 48000
buffer_size = 512
low_latency = false
backend = "auto"
```

### Minimal App Usage

```rust
use engine_core::{AudioBackend, Engine, EngineConfig, FileAudioSource};

// Optional: discover backends/devices before building config
let names = AudioBackend::list_names();
let backend = AudioBackend::new("cpal")?;
let devices = backend.list_output_devices()?;

let cfg = EngineConfig::from_toml_file("config.toml")
    .unwrap_or_default();
let mut engine = Engine::new(cfg)?;
engine.start()?; // opens stream, warms producer, starts playback
engine.load_track(0, &FileAudioSource::new("trackA.mp3"))?;
engine.play(0)?;
engine.pause(0)?;
engine.stop()?;
```

See `app-example` and `README.md` for the runnable reference.

---

_This tech spec describes the modular Rust audio engine architecture. It is kept aligned with the current workspace layout, `AudioSource` loading model, and runtime-selectable backends (`auto` / `cpal` / `miniaudio` / `null`)._
