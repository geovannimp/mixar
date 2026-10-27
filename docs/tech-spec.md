# Tech Spec — Modular Rust Audio Engine

Runtime contracts for the Mixar engine workspace. Product scope lives in [`PRODUCT.md`](../PRODUCT.md); crate map and day-to-day notes in [`DEVELOPER.md`](../DEVELOPER.md); deck UI phases in [`deck-spec.md`](deck-spec.md). Active work is tracked in [GitHub issues](https://github.com/geovannimp/mixar/issues).

## Table of Contents

- [1 — Overview](#1--overview)
- [2 — Core Runtime Contracts & APIs](#2--core-runtime-contracts--apis)
  - [2.1 audio-core (Types & Traits)](#21-audio-core-types--traits)
  - [2.2 Engine Entry Points (engine-core)](#22-engine-entry-points-engine-core)
  - [2.3 Audio loading (`AudioSource`)](#23-audio-loading-audiosource)
- [3 — Threading & Buffer Model](#3--threading--buffer-model)
- [4 — Config Schema](#4--config-schema)
- [5 — Library Manager (Collections Model)](#5--library-manager-collections-model)

---

## 1 — Overview

- **Project name:** Mixar
- **License:** GPLv3
- **Primary dev / CI platform:** Linux x86_64
- **Delivery form:** Rust crate workspace (`crates/`) + Flutter desktop host (`apps/gui-flutter`)

Headless engine crates provide a reusable audio engine for DJ apps: runtime-selectable backends, modular decks, producer/consumer ring-buffer I/O, pluggable `AudioSource` loading, and one library manager per user.

- The audio engine (`engine-dsp` and the producer thread) sends no telemetry. The desktop host may
  send one anonymous `app launched` event through the opt-in `crates/analytics` crate after a
  successful engine start; it has no access to the library, track data, or controller IDs. The
  event is limited to app version, OS, OS version, architecture, and build mode.

Workspace layout and data-flow diagram: [`DEVELOPER.md`](../DEVELOPER.md#high-level-architecture).

## 2 — Core Runtime Contracts & APIs

### 2.1 audio-core (Types & Traits)

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

### 2.2 Engine Entry Points (engine-core)

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
    pub fn list_names() -> Vec<String>; // e.g. ["null", "cpal"]
    pub fn new(name: &str) -> anyhow::Result<Box<dyn audio_core::AudioBackend>>;
}
```

`load_track` / `load_prepared_track` require a running engine (`start` first). Decks install samples at the source’s native sample rate and resample to the engine/stream rate during playback.

### 2.3 Audio loading (`AudioSource`)

| Type | Crate | Role |
|------|--------|------|
| `AudioSource` | `audio-core` / `library-core` | Trait: `load() -> LoadedAudio` |
| `LoadedAudio` | `audio-core` | Decoded interleaved samples + metadata |
| `PreparedTrackPlayback` | `library` | Library-owned handoff: track id + source + cached decode + loudness |
| `FileAudioSource` | `library-core` | Loads from an arbitrary disk path |

**Preferred host path (GUI):** `LibraryManager::prepare_*_for_playback(&Mutex<…>)` → `Engine::load_prepared_track`. The library owns a bounded LRU decode cache (~512 MiB / 8 tracks; cleared with waveform/stem cache wipe) and waveform ensure; the engine does not re-decode. These APIs take `&Mutex<LibraryManager>` so decode/waveform generation does not hold the library lock.

```rust
let prepared = LibraryManager::prepare_track_for_playback(&library, &track_id)?;
engine.load_prepared_track(0, prepared)?;
```

`Engine::load_track` remains for headless / simple callers that pass an `AudioSource` directly. New origins (HTTP, in-memory bytes, etc.) implement `AudioSource` without changing `Engine` or `engine-dsp`.

## 3 — Threading & Buffer Model

### 3.1 Two-thread Design

#### Load path (control thread)

- Prefer library prepare → `Engine::load_prepared_track` (decode/cache outside the host `AppState` lock).
- Or `Engine::load_track(deck_id, &source)` which calls `AudioSource::load()` then installs samples.
- Engine installs samples on the deck via `Deck::load_audio_samples` (native rate).

#### Producer Thread (engine-controlled)

- Runs `DspEngine::process` for each chunk (decks render/resample, mixer routes). The chunk is always `config.buffer_size`, a multiple of the mixer's 64-frame graph buffer.
- Writes interleaved stereo frames to a lock-free ring buffer for the master bus.
- Paced by the pacing device ring's fill level: production stops once that ring holds about two DSP chunks, which ties it to consumption without running unbounded ahead.

#### Consumer (audio callback)

- Implemented by the backend (`cpal` / `null`). The backend’s real-time callback invokes `AudioCallback::render`.
- `ConsumerCallback` pops samples from the ring buffer into the device output buffer. On underrun it writes silence.
- The callback frame count is the backend's business, not the DSP's: WASAPI shared mode always delivers its device period (480 frames at 48 kHz), so the ring buffer is what reconciles that with the 512-frame DSP chunk. Backends request the nearest size the device accepts rather than refusing to open it.
- **No heap allocations, no mutexes, no blocking** in the callback path.

### 3.2 Ring Buffer & Zero Allocations

- Use a lock-free ring buffer (`rtrb`).
- Preallocate capacity of at least `N * frames_per_buffer * channels` (current multiplier is higher than the minimum `N ≥ 8`) to tolerate producer jitter.
- Prefill with silence before the stream starts so the callback has data immediately.
- **No heap allocations in the audio callback.** All buffers preallocated.

### 3.3 Channel Mapping

Bus audio in DSP is stereo. Each bus maps to a device as either a **stereo pair** or a **mono** channel (1-based indexes; mono folds L+R × 0.5 onto one channel). Multi-bus device routing, including `set_bus_device` / `set_bus_channel_mapping`, is implemented.

**Example:** DDJ-400 mapping where master uses channels 3–4 and cue uses 1–2.

## 4 — Config Schema

Config is loaded via `EngineConfig::from_toml_file` / `to_toml_file`. Fields map directly to the `EngineConfig` struct (flat TOML keys, not a nested `[engine]` table unless you wrap it yourself).

**Conventional paths:** local `config.toml`, or the desktop app data directory for identifier `top.mixar.app`.

```toml
buffer_size = 512
backend = "auto"            # headless/test only: "auto", "cpal", or "null"

[[devices]]
name = "Focusrite USB"
id = "hw:1,0"

[[buses]]
# BusConfig fields: id, name, device, channels, sample_rate, low_latency (see audio-core)
# sample_rate is optional per bus: omit it to follow the device's mix rate.
# low_latency is a per-bus backend hint; a device stream uses it when any bus asks.
```

Each bus carries its own optional `sample_rate` and its own `low_latency` hint. The engine runs its DSP graph at the master bus's rate; a device opened at another rate gets a resampler in the producer so its stream stays time-aligned. Buses sharing one device share one stream, so they must agree on the rate (an explicit rate wins over an unset one, and the stream then runs at that rate for every bus on it), while `low_latency` is the union of what they ask for.

`buffer_size` stays global: it is the mixer chunk the DSP renders in one go, and it doubles as the stream-buffer size requested from each backend (`frames_per_buffer`), which a backend may clamp to the device's supported range. It is not per-device, because the device callback period belongs to the driver and the per-device ring absorbs the difference.

- `backend` is a headless/test selector only: the desktop app always boots the compiled-in real backend (CPAL) and never persists or shows a backend choice.
- `backend = "auto"` resolves to CPAL when it is compiled in; it never falls back to `null`.
- Valid backend names: `"auto"`, `"cpal"`, `"null"`.
- `"null"` is reserved for headless tests and `engine-core` test configs.
- `backend-cpal` is an optional Cargo feature on `engine-core` (enabled by default).

See [`README.md`](../README.md) for a minimal embed example.

## 5 — Library Manager (Collections Model)

The library subsystem is a **generic, import/export-friendly model**: one internal representation that external formats can map into and out of.

Playback: `Track` implements `AudioSource`, so load with `engine.load_track(deck_id, &track)`. The library never owns the audio device path.

### 5.1 Core concepts

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
    // reserved for later:
    | SmartPlaylist
    | History
```

**Shipped types:** `Folder` and `Playlist` only. **No playlist folders** (virtual folders that only nest playlists).

| Type | Meaning | How tracks are associated |
|------|---------|---------------------------|
| `Folder` | Points at a **real directory** on disk (`fs_path` required). Used as a scan root and to browse music under that path. | **By path:** tracks whose `path` is under `fs_path`. **Not** via `collection_entries`. |
| `Playlist` | Named list of tracks (ordered or not). | **Many-to-many** via `collection_entries`. |

**`sortable` on playlist collections only:**

- `sortable == true`: membership rows carry a `position`; `reorder_tracks` is allowed.
- `sortable == false`: membership is a set (“crate”-style); `position` is null.

Toggling `sortable` from `false` → `true` assigns positions; `true` → `false` drops positions.

**Browsing a disk folder:** when `respect_folder_tree` is enabled, the UI may show **real subdirectories** under `fs_path` by reading the filesystem (or grouping tracks by path prefix). Those subdirs are **not** stored as separate Collection rows unless the user explicitly adds them as Folder collections. Playlists are never children of folders in the database.

#### Explicit non-goals (for now)

- Virtual playlist folders (nodes that only organize playlists).
- Hierarchy encoded only in collection names.
- Nesting playlists under folders in the schema (`parent_id` tree of collections).

### 5.2 Canonical data model

Tracks and collections are **separate entities**.

- **Folder → tracks:** association by **filesystem path** (`track.path` under `collection.fs_path`). No join table.
- **Playlist → tracks:** **many-to-many** via `collection_entries`. One track can be in many playlists; one playlist has many tracks.

```text
Library
├── tracks: Map<TrackId, Track>              // own table
├── collections: Map<CollectionId, Collection>  // folders + playlists (flat)
├── collection_entries: Set<CollectionEntry>  // M2M for playlists only
└── config: LibraryConfig
```

```text
Folder collection                    Playlist collection
  fs_path = /Music/House               id = "pl-warmup"
       │                                    │
       │ path prefix                        │ M2M
       ▼                                    ▼
  tracks.path LIKE '/Music/House/%'   collection_entries
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
pub struct CollectionEntry {
    pub id: CollectionEntryId,
    pub collection_id: CollectionId,
    pub track_id: TrackId,
    pub position: Option<i32>,
}
```

**Invariants:**

- Tracks live only in the track pool; collections never embed track metadata.
- `Folder` collections **must** have `fs_path` set to an existing (or user-chosen) directory path.
- `Playlist` collections **must not** have `fs_path`.
- Only `Playlist` collections appear in `collection_entries`.
- Tracks “in” a folder = `track.path` is under that folder’s `fs_path` (path-prefix query).
- Unsortable playlists (crates) treat membership as a set: at most one `(collection_id, track_id)` row.
- Sortable playlists may list the same track more than once (separate `collection_entries` rows).
- No `parent_id` / collection tree — collections are a **flat** list (playlist folders unsupported).
- Deleting a Folder collection does **not** delete tracks (they may still sit under that path; optional policy: leave tracks or mark orphaned).
- Deleting a playlist removes only its `collection_entries` rows; tracks remain.
- Deleting a track removes all of its `collection_entries` rows; the audio file on disk is never deleted by the library.

### 5.3 Import / export policy

Importers **normalize into the user’s one Library**. Virtual playlist-folder trees are **flattened** on import. On export, `sortable: false` playlists become crates where the target has crates; `sortable: true` become playlists. Per-format mapping notes live outside the repo (private research).

### 5.4 Capability traits and backends

```text
library-core/       # types + traits (no I/O)
library/            # library manager (canonical writable store)
library-adapters/   # third-party formats (modules / features over time)
```

| Trait | Responsibility |
|-------|----------------|
| `Library` (read) | List/get tracks; list collections; tracks under a folder (by path); tracks in a playlist (M2M) |
| `WritableLibrary` | `add_collection` / `sync_collection`; edit playlist membership; set `sortable` |
| `Migratable` | Copy tracks + collections into a `WritableLibrary` (typically native) |
| `Exportable` | Write a target format (Rekordbox XML, CDJ USB layout, M3U, …) from a `Library` |
| `ExportSource` | Analysis/waveform/cue access a format needs beyond `Library`; implemented by `LibraryManager` |

`library` (`LibraryManager`) implements `Library` + `WritableLibrary` and
`ExportSource`. Persistence is an implementation detail of that crate. Adapters
in `library-adapters` implement `Library` + `Migratable`, and optionally
`Exportable` / limited write-back. Migration always targets the **user’s one
library**, not a second parallel manager.

Shipped: the Mixxx importer ([#57](https://github.com/geovannimp/mixar/issues/57))
and the CDJ/XDJ USB exporter ([#452](https://github.com/geovannimp/mixar/issues/452),
`library-adapters::CdjExporter`): DeviceSQL `export.pdb` (tracks, playlists, artwork
rows) plus ANLZ analysis files and cover thumbnails under `PIONEER/`, audio below
`Contents/`. Hosts hand the exporter
`library::ExportSession`, which holds the manager lock only for database reads
and decode-cache access — decode, DSP and cover encoding run without it — so
browsing stays responsive during a long run. Format details, placement rules and
the fidelity list live in
[`library-adapters/README.md`](../crates/library-adapters/README.md).

```text
External format          User library manager            External format
(Rekordbox XML, …)  ──►  (library / LibraryManager)  ──►  (CDJ / XML / M3U / …)
     Migratable                 ▲  │                        Exportable
     (library-adapters)         │  │
                         scan / tags / UI edits
```

Adapter work (Rekordbox, Serato, Traktor, CDJ export, tag write-back): [#57](https://github.com/geovannimp/mixar/issues/57), [#251](https://github.com/geovannimp/mixar/issues/251), [#451](https://github.com/geovannimp/mixar/issues/451), [#452](https://github.com/geovannimp/mixar/issues/452).

### 5.5 Persistent storage architecture

Three tables (current engine uses a local SQL database; that choice is an implementation detail of `library`). Collections are a **flat** list (no `parent_id` tree).

```text
Folder: tracks linked by path prefix          Playlist: tracks linked by M2M

  collections (type=folder)                     collections (type=playlist)
       fs_path = /Music/House                        id = pl1
            │                                         │
            │ track.path starts with fs_path          │ collection_entries
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
  duration_ms INTEGER,
  sample_rate INTEGER,
  channels INTEGER,
  bitrate_kbps INTEGER,
  rating INTEGER,          -- 1-5, NULL = unrated
  comment TEXT,
  year INTEGER,
  track_number INTEGER,
  disc_number INTEGER,
  color_label TEXT,        -- pink | red | orange | yellow | green | aqua | blue | purple
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
-- Row `id` is the primary key so sortable playlists may list a track more than once.
CREATE TABLE collection_entries (
  id TEXT PRIMARY KEY NOT NULL,
  collection_id TEXT NOT NULL REFERENCES collections(id) ON DELETE CASCADE,
  track_id TEXT NOT NULL REFERENCES tracks(id) ON DELETE CASCADE,
  position INTEGER
);

CREATE INDEX idx_collection_entries_track ON collection_entries(track_id);
CREATE INDEX idx_collection_entries_collection_pos
  ON collection_entries(collection_id, position);
```

**Tracks in a folder (query, not join table):**

```sql
SELECT t.* FROM tracks t
JOIN collections c ON c.id = ? AND c.collection_type = 'folder'
WHERE t.path = c.fs_path OR t.path LIKE c.fs_path || '/%';
```

DJ fields (hot cues, loops, waveforms, stems) use dedicated tables in the same `library.db` (see `library` crate). File-tag read on import uses lofty (via `codec`); optional tag write-back of BPM/key (`LibraryConfig::tag_write_back`, off by default) uses the same crate — see [`audio-analyzer-spec.md` §11.6](audio-analyzer-spec.md#116--tag-write-back-optional) for the eligible fields and per-format lossiness.

### 5.6 Design decisions (summary)

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Manager | One `Library` per user | Single place for tracks, collections, adapters |
| Collection kinds | `Folder` (disk path) + `Playlist` (`sortable`) | Real folders + lists; no virtual playlist folders |
| Folder membership | Path prefix on `tracks.path` | Folder is a real directory, not a join table |
| Playlist membership | M2M `collection_entries` | Same track in many playlists |
| Collection layout | Flat list (no `parent_id`) | Playlist folders out of scope |
| Ordered vs unordered lists | `Playlist.sortable` | Playlist and crate are the same structure |
| Canonical store | `library` / `LibraryManager` | Mixxx-like reliability; storage engine is an implementation detail |
| External apps | `library-adapters` + `Migratable` / `Exportable` | No coupling of proprietary parsers to the manager schema |
| Playback | `PreparedTrackPlayback` → `load_prepared_track` (or `AudioSource` → `load_track`) | Library owns decode/metadata; engine owns the device path |
