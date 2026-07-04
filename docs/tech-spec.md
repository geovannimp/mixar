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
- [10 — Library Manager (Collections Model)](#10--library-manager-collections-model)
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

- **Library manager** — one `Library` per user, holding tracks and typed `Collection`s (disk `folder` | `playlist` with `sortable`); import/export adapters; see §10.

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
├─ library-core/       # Library traits + Collection/Track types
├─ library/            # library manager (canonical writable store)
├─ library-adapters/   # third-party formats (Rekordbox, Serato, …)
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
| `Track` | `library-core` | Library entry; implements `AudioSource` (loads `Track::path` via `codec`) |
| `FileAudioSource` | `engine-core` | Loads from an arbitrary disk path; also `FileAudioSource::from_track(&track)` |

Callers never pass a bare path to `Engine::load_track`. Prefer a library `Track` when available:

```rust
engine.load_track(0, &track)?;
// or, for a path outside the library:
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

## 10 — Library Manager (Collections Model)

The library subsystem is a **generic, import/export-friendly model** inspired primarily by [Mixxx](https://mixxx.org/) and aligned with how major DJ applications organize tracks. The goal is one internal representation that any external format (Rekordbox, Serato, Traktor, VirtualDJ, Engine DJ, CDJ USB, M3U, …) can map into and out of.

Playback: `Track` implements `AudioSource`, so load with `engine.load_track(deck_id, &track)`. The library never owns the audio device path.

### 10.1 Survey of rival library models

| Application | Track store | Organization model | Hierarchy | Ordered lists | Unordered sets | Interchange format |
|-------------|-------------|-------------------|-----------|---------------|----------------|--------------------|
| **Mixxx** | SQLite (`library` + `track_locations`) | Playlists + crates; watched directories | Playlists/crates are mostly flat (users want folders) | Playlists (`position`) | Crates | External importers (Rekordbox, Serato, Traktor, iTunes, …) |
| **Rekordbox** | Track pool + playlist tree | **NODE with Type** | Nested folders | Playlists (`Type=1`) | — | XML export (`DJ_PLAYLISTS`); proprietary DB |
| **Serato** | `database V2` (binary TLV) | Crates (`.crate` files) | Encoded in crate **name** (`Parent%%Child`) | — | Crates | Proprietary; path-based track refs |
| **Traktor** | `collection.nml` (XML) | Playlist folders + playlists | Nested playlist folders | Playlists | — | NML; playlist export folders |
| **VirtualDJ** | Local DB + **My Lists** XML | Folders + lists | Browser folders + list tree | Lists (`ordered`) | Virtual folders | VDJ List XML, M3U, CDJ export |
| **Engine DJ** | SQLite (`m.db`, …) | Crates + playlists | Crate/playlist **path** (parent titles) | Playlists | Crates | Engine Library on USB; third-party attach DB |

**What we take from rivals (and what we deliberately skip):**

| Idea | Source | Our model |
|------|--------|-----------|
| One library manager + track pool | Mixxx | **Library** |
| Watched **disk** directories | Mixxx `directories` | **Collection `Folder`** — always a real path on disk |
| Playlists / crates as track lists | Mixxx, Rekordbox, … | **Collection `Playlist`** + `sortable` |
| Virtual playlist folders (folders that only group playlists) | Rekordbox / Traktor / Serato crate hierarchy | **Out of scope for now** — not supported |

This is **not** Serato/Rekordbox-style virtual hierarchy. A Folder is never a logical container for playlists; it is a pointer to a directory on the user’s computer (scan + browse by path).

### 10.2 Core concepts

#### Library vs Collection vs Track

| Term | Meaning |
|------|---------|
| **Library** | The **manager**. Each user has **one** library. It owns the track pool, collections, and persistence. Backends implement `Library` / `WritableLibrary` (`library-core`). |
| **Collection** | A typed entry in the library: either a **disk folder** or a **playlist**. A library holds **many** collections (flat list — no playlist-folder tree). |
| **Track** | One audio file entry (path + metadata + optional DJ fields). Lives in the library’s track pool. |

```text
User
 └── Library (manager — one per user)
      ├── tracks: shared pool (each track has a filesystem path)
      ├── collections (flat list, different types)
      │    ├── Folder  fs_path = /home/me/Music/House
      │    ├── Folder  fs_path = /home/me/Music/Techno
      │    ├── Playlist "Warmup"   (sortable: true)
      │    └── Playlist "Favorites" (sortable: false)  // crate-like
      └── config (e.g. respect_folder_tree for UI under a disk folder)
```

#### Collection types

```text
CollectionType =
    | Folder      // real disk directory; fs_path is required
    | Playlist    // user-defined track list; order controlled by `sortable`
    // reserved for later (not MVP):
    | SmartPlaylist
    | History
```

**MVP types:** `Folder` and `Playlist` only. **No playlist folders** (virtual folders that only nest playlists).

| Type | Meaning | How tracks are associated |
|------|---------|---------------------------|
| `Folder` | Points at a **real directory** on disk (`fs_path` required). Used as a scan root and to browse music under that path. | **By path:** tracks whose `path` is under `fs_path`. **Not** via `collection_tracks`. |
| `Playlist` | Named list of tracks (ordered or not). | **Many-to-many** via `collection_tracks`. |

**`sortable` on playlist collections only:**

- `sortable == true`: membership rows carry a `position`; `reorder_tracks` is allowed.
- `sortable == false`: membership is a set (Mixxx/Serato/Engine “crate”); `position` is null.

Toggling `sortable` from `false` → `true` assigns positions; `true` → `false` drops positions.

**Browsing a disk folder:** when `respect_folder_tree` is enabled, the UI may show **real subdirectories** under `fs_path` by reading the filesystem (or grouping tracks by path prefix). Those subdirs are **not** stored as separate Collection rows unless the user explicitly adds them as Folder collections. Playlists are never children of folders in the database.

#### Explicit non-goals (for now)

- Virtual playlist folders (Rekordbox/Traktor “folder” nodes that only organize playlists).
- Serato-style hierarchy encoded in names (`Parent%%Child`).
- Nesting playlists under folders in the schema (`parent_id` tree of collections).

### 10.3 Canonical data model

Tracks and collections are **separate entities**.

- **Folder → tracks:** association by **filesystem path** (`track.path` under `collection.fs_path`). No join table.
- **Playlist → tracks:** **many-to-many** via `collection_tracks`. One track can be in many playlists; one playlist has many tracks.

```text
Library
├── tracks: Map<TrackId, Track>              // own table
├── collections: Map<CollectionId, Collection>  // folders + playlists (flat)
├── collection_tracks: Set<CollectionTrack>  // M2M for playlists only
└── config: LibraryConfig
```

```text
Folder collection                    Playlist collection
  fs_path = /Music/House               id = "pl-warmup"
       │                                    │
       │ path prefix                        │ M2M
       ▼                                    ▼
  tracks.path LIKE '/Music/House/%'   collection_tracks
```

```rust
// Conceptual types (library-core)

pub struct TrackId(/* opaque, stable within a library */);
pub struct CollectionId(/* opaque, stable within a library */);

/// Standalone track entity. Not owned by any collection.
pub struct Track {
    pub id: TrackId,
    pub path: PathBuf,
    pub metadata: TrackMetadata,
    pub dj: DjMetadata,
}

pub enum CollectionType {
    Folder,
    Playlist,
}

pub struct Collection {
    pub id: CollectionId,
    pub name: String,
    pub collection_type: CollectionType,
    /// Display order in the library’s collection list.
    pub sort_index: i32,
    /// Playlist only: ordered vs set. Unused for Folder.
    pub sortable: bool,
    /// Folder only: absolute path to a real directory on disk. Required for Folder; None for Playlist.
    pub fs_path: Option<PathBuf>,
}

/// Many-to-many join: playlist ↔ track only.
pub struct CollectionTrack {
    pub collection_id: CollectionId,
    pub track_id: TrackId,
    pub position: Option<i32>,
}
```

**Invariants:**

- Tracks live only in the track pool; collections never embed track metadata.
- `Folder` collections **must** have `fs_path` set to an existing (or user-chosen) directory path.
- `Playlist` collections **must not** have `fs_path`.
- Only `Playlist` collections appear in `collection_tracks`.
- Tracks “in” a folder = `track.path` is under that folder’s `fs_path` (path-prefix query).
- `(collection_id, track_id)` is unique in `collection_tracks`.
- No `parent_id` / collection tree — collections are a **flat** list (playlist folders unsupported).
- Deleting a Folder collection does **not** delete tracks (they may still sit under that path; optional policy: leave tracks or mark orphaned).
- Deleting a playlist removes only its `collection_tracks` rows; tracks remain.
- Deleting a track removes all of its `collection_tracks` rows; the audio file on disk is never deleted by the library.

### 10.4 Mapping rivals → canonical model

| Source concept | Maps to |
|----------------|---------|
| Mixxx watched directory | `Collection { type: Folder, fs_path }` |
| Mixxx playlist | `Collection { type: Playlist, sortable: true }` + `collection_tracks` |
| Mixxx crate | `Collection { type: Playlist, sortable: false }` + `collection_tracks` |
| Rekordbox / Traktor / VDJ **playlist** | `Playlist { sortable: true }` (flat — no parent folder) |
| Rekordbox / Traktor **playlist folder** | **Dropped** on import (playlists promoted to top-level; folder names may be prefixed onto playlist name if useful) |
| Rekordbox track / path | `Track` in the pool |
| Serato `.crate` | `Playlist { sortable: false }` (hierarchy in crate names **flattened**) |
| Engine crate / playlist | `Playlist` with appropriate `sortable` (path hierarchy flattened) |
| Real disk path on any system | `Folder` only when it is an actual directory the user (or importer) registers |
| M3U / M3U8 | `Playlist { sortable: true }` + tracks by path |
| CDJ USB | export playlists + files; folder collections export as real directories of files if needed |

Importers **normalize into the user’s Library**. Virtual playlist-folder trees are **flattened**. On export, `sortable: false` playlists become crates where the target has crates; `sortable: true` become playlists. We do not invent Serato-style `Parent%%Child` names unless an exporter explicitly chooses to.

### 10.5 Capability traits and backends

Workspace layout (mirrors audio backends):

```text
library-core/       # types + traits (no I/O)
library/            # library manager (canonical writable store)
library-adapters/   # third-party formats (modules / features over time)
  rekordbox/        # planned
  serato/           # planned
  traktor/          # planned
  virtualdj/        # planned
  engine/           # planned
```

Traits (compose per backend):

| Trait | Responsibility |
|-------|----------------|
| `Library` (read) | List/get tracks; list collections; tracks under a folder (by path); tracks in a playlist (M2M) |
| `WritableLibrary` | `add_collection` / `sync_collection`; edit playlist membership; set `sortable` |
| `Migratable` | Copy tracks + collections into a `WritableLibrary` (typically native) |
| `Exportable` | Write a target format (Rekordbox XML, CDJ USB layout, M3U, …) from a `Library` |

`library` (`LibraryManager`) implements `Library` + `WritableLibrary`. Persistence is an implementation detail of that crate. Adapters in `library-adapters` implement `Library` + `Migratable`, and optionally `Exportable` / limited write-back. Migration always targets the **user’s one library**, not a second parallel manager.

### 10.6 Tag reading and DJ metadata

- **Primary read on import:** file tags (ID3, Vorbis, FLAC, …) via lofty (or equivalent).
- **Canonical store:** the `library` crate persists tracks, collections, membership, and app-managed DJ fields (storage engine is an implementation detail).
- **Optional write-back** to file tags (v1+), format-dependent.
- **DJ fields** (hotcues, beatgrid, rating, color): stored on `Track`, not on Collection nodes. Phased: MVP may ship tracks + collections only; cues/grid follow without changing the Collection type model.

### 10.7 Import / export flows

```text
External format          User library manager            External format
(Rekordbox XML, …)  ──►  (library / LibraryManager)  ──►  (CDJ / XML / M3U / …)
     Migratable                 ▲  │                        Exportable
     (library-adapters)         │  │
                         scan / tags / UI edits
```

1. **Open external** → read-only `Library` view via `library-adapters` (browse without copying into the user’s store).
2. **Migrate to user library** → optional full copy into `LibraryManager` (user-driven).
3. **Export** from any `Library` (native or external adapter) via `Exportable` when implemented.
4. **Round-trip fidelity:** best-effort; document per-adapter lossiness (e.g. smart playlists → static playlist snapshot).

### 10.8 Persistent storage architecture

Three tables (current engine uses a local SQL database; that choice is an implementation detail of `library`). Collections are a **flat** list (no `parent_id` tree).

```text
Folder: tracks linked by path prefix          Playlist: tracks linked by M2M

  collections (type=folder)                     collections (type=playlist)
       fs_path = /Music/House                        id = pl1
            │                                         │
            │ track.path starts with fs_path          │ collection_tracks
            ▼                                         ▼
         tracks                                    tracks
```

```sql
-- Track pool: one row per audio file.
CREATE TABLE tracks (
  id TEXT PRIMARY KEY,
  path TEXT NOT NULL UNIQUE,
  title TEXT,
  artist TEXT,
  album TEXT,
  genre TEXT,
  bpm REAL,
  key TEXT,
  duration_secs REAL,
  sample_rate INTEGER,
  channels INTEGER,
  bitrate_kbps INTEGER,
  added_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE INDEX idx_tracks_path ON tracks(path);

-- Flat list of collections: disk folders and playlists.
-- Folder:  collection_type = 'folder',  fs_path NOT NULL, sortable ignored
-- Playlist: collection_type = 'playlist', fs_path IS NULL, sortable 0|1
CREATE TABLE collections (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  collection_type TEXT NOT NULL,  -- 'folder' | 'playlist'
  sort_index INTEGER NOT NULL DEFAULT 0,
  sortable INTEGER NOT NULL DEFAULT 1,
  fs_path TEXT,                  -- required for folder; NULL for playlist
  UNIQUE (fs_path)               -- one collection per disk path (NULL fs_path allowed for playlists)
);

-- Many-to-many: playlist ↔ track only (never folder ids).
CREATE TABLE collection_tracks (
  collection_id TEXT NOT NULL REFERENCES collections(id) ON DELETE CASCADE,
  track_id TEXT NOT NULL REFERENCES tracks(id) ON DELETE CASCADE,
  position INTEGER,
  PRIMARY KEY (collection_id, track_id)
);

CREATE INDEX idx_collection_tracks_track ON collection_tracks(track_id);
CREATE INDEX idx_collection_tracks_collection_pos
  ON collection_tracks(collection_id, position);
```

**Tracks in a folder (query, not join table):**

```sql
SELECT t.* FROM tracks t
JOIN collections c ON c.id = ? AND c.collection_type = 'folder'
WHERE t.path = c.fs_path OR t.path LIKE c.fs_path || '/%';
```

**Lifecycle examples:**

| Action | Effect |
|--------|--------|
| `add_collection` (folder) | Insert `collections` row with `fs_path` |
| `sync_collection` (folder) | Walk `fs_path`; insert/update `tracks` for audio files found |
| List folder tracks | Path-prefix query on `tracks` (no `collection_tracks`) |
| `add_collection` (playlist) | Insert `collections` row (`type=playlist`, `fs_path` NULL) |
| `sync_collection` (playlist) | Refresh tags for member tracks from disk |
| Add track to playlist | Insert `collection_tracks` row |
| Remove track from playlist | Delete one `collection_tracks` row |
| Delete playlist | Cascade-delete its `collection_tracks` rows; `tracks` unchanged |
| Delete folder collection | Delete `collections` row only; `tracks` unchanged |
| Delete track from library | Delete `tracks` row; cascade-delete its `collection_tracks` rows |

### 10.9 API sketch

```rust
trait Library: Send + Sync {
    fn name(&self) -> &'static str;

    fn list_tracks(&self, query: &TrackQuery) -> Result<Vec<Track>>;
    fn get_track(&self, id: &TrackId) -> Result<Option<Track>>;

    fn list_collections(&self) -> Result<Vec<Collection>>;
    fn get_collection(&self, id: &CollectionId) -> Result<Option<Collection>>;

    /// Tracks under a Folder collection (path-prefix). Errors if not a Folder.
    fn tracks_in_folder(&self, folder_id: &CollectionId) -> Result<Vec<Track>>;
    /// Tracks in a Playlist collection (M2M). Errors if not a Playlist.
    fn tracks_in_playlist(&self, playlist_id: &CollectionId) -> Result<Vec<Track>>;
}

trait WritableLibrary: Library {
    fn import_path(&mut self, path: &Path) -> Result<Track>;

    /// Add a collection (folder or playlist) via [`NewCollection`].
    fn add_collection(&mut self, collection: &NewCollection) -> Result<Collection>;
    /// Sync one collection (or all when `None`). Folders rescan disk; playlists refresh member tags.
    fn sync_collection(&mut self, collection_id: Option<&CollectionId>) -> Result<ScanReport>;

    fn rename_collection(&mut self, id: &CollectionId, name: &str) -> Result<()>;
    fn delete_collection(&mut self, id: &CollectionId) -> Result<()>;
    fn set_sortable(&mut self, playlist_id: &CollectionId, sortable: bool) -> Result<()>;

    fn add_to_playlist(&mut self, playlist_id: &CollectionId, track_id: &TrackId, position: Option<i32>) -> Result<()>;
    fn remove_from_playlist(&mut self, playlist_id: &CollectionId, track_id: &TrackId) -> Result<()>;
    /// Errors if the playlist is not `sortable`.
    fn reorder_tracks(&mut self, playlist_id: &CollectionId, track_ids: &[TrackId]) -> Result<()>;
}
```

### 10.10 Implementation status and roadmap

**Current code:** `library-core` / `library` implement the §10 model (disk `Folder` collections, `Playlist` + `sortable`, path-prefix folder tracks, M2M `collection_tracks`). `library-adapters` is a placeholder.

**Next implementation steps:**

1. Adapters in `library-adapters` (Rekordbox XML first; flatten virtual playlist folders; crates ↔ `Playlist { sortable: false }`).
2. DJ metadata (hotcues, beatgrid) on `Track`.
3. Optional tag write-back and CDJ export.

### 10.11 Design decisions (summary)

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Manager | One `Library` per user | Single place for tracks, collections, adapters |
| Collection kinds | `Folder` (disk path) + `Playlist` (`sortable`) | Real folders + lists; no virtual playlist folders |
| Folder membership | Path prefix on `tracks.path` | Folder is a real directory, not a join table |
| Playlist membership | M2M `collection_tracks` | Same track in many playlists |
| Collection layout | Flat list (no `parent_id`) | Playlist folders out of scope |
| Ordered vs unordered lists | `Playlist.sortable` | Playlist and crate are the same structure |
| Canonical store | `library` / `LibraryManager` | Mixxx-like reliability; storage engine is an implementation detail |
| External apps | `library-adapters` + `Migratable` / `Exportable` | No coupling of proprietary parsers to the manager schema |
| Playback | `Track: AudioSource` → `Engine::load_track` | Library does not own the audio device path |

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
- Library manager: disk `Folder` collections + `Playlist` (`sortable`); track scan/tags.
- Latency reporting / buffer negotiation polish.

### v1

- Hotcues/beatgrid on tracks; optional tag write-back.
- Rekordbox XML import/export adapter; CDJ playlist export.
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

### Sprint 3 — Library Manager & Tags (in progress)

- `library-core` / `library` (`LibraryManager`): disk `Folder` + `Playlist` (`sortable`), path-prefix folder tracks, M2M `collection_tracks` (§10).
- `library-adapters`: placeholder for third-party formats.
- Later sprints: adapters (crates → `sortable: false`), hotcues/beatgrid, CDJ export.

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

_This tech spec describes the modular Rust audio engine architecture. It is kept aligned with the current workspace layout, `AudioSource` loading model, runtime-selectable backends (`auto` / `cpal` / `miniaudio` / `null`), and the library manager / collections model in §10._
