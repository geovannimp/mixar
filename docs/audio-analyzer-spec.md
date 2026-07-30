# Tech Spec — Audio Analyzer Crate

Reference: main engine spec [`tech-spec.md`](tech-spec.md) §3, §10. Related crates: `codec`, `library`, `engine-dsp`.

## Table of Contents

- [1 — Overview](#1--overview)
- [2 — Objectives](#2--objectives)
- [3 — Non-goals](#3--non-goals)
- [4 — Relationship to Existing Code](#4--relationship-to-existing-code)
- [5 — Backend Evaluation](#5--backend-evaluation)
- [6 — Architecture](#6--architecture)
- [7 — Analysis Pipeline](#7--analysis-pipeline)
- [8 — Core Types & API](#8--core-types--api)
- [9 — Library Integration](#9--library-integration)
- [10 — Engine / Playback Integration](#10--engine--playback-integration)
- [11 — Storage Schema (Library DB)](#11--storage-schema-library-db)
- [12 — Performance & Resource Limits](#12--performance--resource-limits)
- [13 — Testing & Acceptance Criteria](#13--testing--acceptance-criteria)
- [14 — Roadmap (Phases)](#14--roadmap-phases)
- [15 — Design Decisions (Summary)](#15--design-decisions-summary)

---

## 1 — Overview

- **Purpose:** Offline, file-based audio analysis for DJ workflows — BPM, musical key, and beat grid generation.
- **Delivery form:** Small workspace crate(s) with a stable trait boundary (`analyzer-core`) and pluggable backends.
- **Primary consumer:** `library` (`WritableLibrary::analyze_track`, folder sync, batch re-analysis).
- **License compatibility:** Project is GPLv3; preferred backends must allow linking (MIT/Apache-2.0 OK).

Analysis answers: *“What tempo, key, and beat positions does this track have?”* — the same problem space as Mixed In Key, Rekordbox analysis, and Serato auto-BPM.

---

## 2 — Objectives

### MVP (Phase 1)

- **Offline analysis** of local audio files (same formats as `codec` / symphonia).
- **BPM** with confidence score; honor DJ range (e.g. 60–200 BPM) and half/double disambiguation policy.
- **Key** in **musical notation** (`C#m`, `Bb`, `F major`) with confidence. Camelot / Open Key (`8A`, `12B`) may be produced by backends internally but is **never** stored or returned by the library (see §9.4).
- **Beat grid:** beat timestamps (seconds), bar/downbeat markers, grid stability metric.
- **Stable Rust API** decoupled from any single backend; errors are typed and backend-agnostic.
- **Headless / CI-friendly:** no GUI, no ONNX required for default build; runs in unit tests with short WAV fixtures.
- **Integration hook** for `library::analyze_track` to fill `TrackMetadata.bpm` / `TrackMetadata.key` and persist beat grid.

### Phase 2+

- Optional **ML refinement** (stratum-dsp `ml` / ONNX feature) behind a Cargo feature.
- **Progress / cancellation** for long batch jobs.
- **Tag write-back** (lofty) when user opts in.
- **Secondary backend** for features stratum-dsp does not cover (see §5).
- **Waveform overviews** (peak/RMS envelope) — separate module or crate; not required for MVP.

---

## 3 — Non-goals

- **Real-time deck BPM** during playback — BPM/key/beat grid come from library track metadata (offline `analyzer-*` crate), not live buffer analysis.
- **Time-stretch / pitch-shift** — future `engine-dsp` extension.
- **Stem separation, genre/mood classification, chord recognition** — out of MVP; optional via secondary backend later.
- **Cloud analysis services** — local-only for privacy and offline libraries.
- **Replacing tag reading** — file tags remain the fast path; analysis supplements or overrides stale tags.
- **WASM analysis in the browser** — possible later; not MVP (stratum-dsp pulls symphonia/rayon; evaluate separately).

---

## 4 — Relationship to Existing Code

| Component | Role today | After analyzer |
|-----------|------------|----------------|
| `library/src/tags.rs` | Reads ID3/Vorbis tags via `lofty` (incl. BPM/key from tags) | Still used first; analyzer runs when forced or tags missing/low confidence |
| `library::analyze_track` | Re-reads tags + audio properties only | Full DSP analysis + DB persist |
| `TrackMetadata` (`library-core`) | `bpm`, `key` optional fields | `key` is always musical notation; beat grid stored separately (§11) |
| `engine-dsp` | Playback DSP only | Uses BPM/key/beat grid from loaded track metadata, not live analysis |
| `codec` | Decode for playback | Reused to produce mono `f32` analysis buffer |

```text
Import / analyze_track
        │
        ▼
   lofty (tags) ──► TrackMetadata (title, artist, tag BPM/key, …)
        │
        ▼
   codec (decode) ──► mono f32 PCM
        │
        ▼
   analyzer backend ──► TrackAnalysis (BPM, key, beat grid, confidences)
        │
        ▼
   library DB persist
```

---

## 5 — Backend Evaluation

### 5.1 Recommended primary: [stratum-dsp](https://docs.rs/stratum-dsp/latest/stratum_dsp/) 1.x

Pure Rust, DJ-oriented, zero FFI in default build. MIT OR Apache-2.0.

| Capability | Support | Notes |
|------------|---------|-------|
| BPM + confidence | Yes | Dual tempogram (FFT + autocorrelation), comb filterbank |
| Key (musical) | Yes | Krumhansl–Kessler chroma templates; map stratum `Key` → musical string for library |
| Key (Camelot, internal) | Yes | Backend-only; not persisted in library |
| Beat grid | Yes | `BeatGrid { beats, bars, downbeats }` in seconds |
| Tempo drift | Yes | HMM beat tracking with drift correction |
| Grid stability metric | Yes | `grid_stability` on result |
| Serde results | Yes | `AnalysisResult` serializable |
| Documented DJ benchmark | Yes | ~87.7% within ±2 BPM, ~72.1% key exact match on 155 Beatport/ZipDJ tracks ([crate README](https://crates.io/crates/stratum-dsp)) |
| Optional ML (ONNX) | Phase 2 feature | `ort` optional dependency |

**Gaps vs ideal DJ stack:**

- BPM accuracy below commercial Mixed In Key on vendor benchmarks (~98% ±2 BPM MIK vs ~88% stratum-dsp) — acceptable for MVP; ML feature may close gap.
- No standalone **energy/loudness curve** or **phrase/section** labels (intro/drop) — not required for beat sync MVP.
- **symphonia** dependency overlap with our `codec` crate — we decode once in `analyzer` and pass samples to avoid double policy drift.
- Analysis expects **mono `f32`**, normalized; stereo must be downmixed upstream.

**Verdict:** Default backend for Phase 1.

### 5.2 Optional secondary: [oximedia-mir](https://crates.io/crates/oximedia-mir) 0.1.x

Pure Rust MIR suite (Apache-2.0), part of the OxiMedia workspace.

| Capability | Support | Notes |
|------------|---------|-------|
| Tempo / beat | Yes | Autocorrelation, comb filtering, DP beat tracker |
| Downbeat | Yes | Stronger explicit downbeat/phrase path than stratum-dsp docs emphasize |
| Key | Yes | Krumhansl–Schmuckler |
| Chords, melody, structure, genre, mood | Yes | Feature-gated; broader than DJ MVP |
| DJ validation / Camelot | Limited | Less DJ-specific; smaller community footprint |
| Maturity | Early | ~0.1.x, fewer downloads |

**Verdict:** Phase 2+ backend behind `analyzer-oximedia` for **downbeat/structure** features if stratum-dsp grid quality is insufficient on internal fixtures. Not default.

### 5.3 Considered but not recommended for MVP

| Library | Reason to defer |
|---------|-----------------|
| [bpm-analyzer](https://crates.io/crates/bpm-analyzer) | Real-time / CPAL capture focus; BPM only; no key/grid |
| **aubio** (FFI) | C dependency, complicates CI and cross-compile |
| **Essentia** (FFI) | Powerful but heavy C++ stack; licensing/build cost |
| **librosa** (Python) | Out of process; not suitable for embedded library crate |

### 5.4 Feature matrix (decision aid)

| Feature | stratum-dsp | oximedia-mir | MVP need |
|---------|-------------|--------------|----------|
| BPM | ✓ | ✓ | **Required** |
| Key (musical) | ✓ | ✓ | **Required** (library canonical form) |
| Key (Camelot) | internal | — | UI may convert at display time; not stored |
| Beat timestamps | ✓ | ✓ | **Required** |
| Bar/downbeat grid | ✓ | ✓ | **Required** |
| Confidence scores | ✓ | varies | **Required** |
| Pure Rust, no FFI | ✓ | ✓ | **Required** |
| ONNX refinement | optional | — | Phase 2 |
| Chord/structure | — | ✓ | Defer |
| Real-time tap tempo | — | — | `engine-dsp` |

---

## 6 — Architecture

### 6.1 Workspace layout

```text
rust-dj-engine/
├─ analyzer-core/        # Traits, TrackAnalysis types, AnalysisConfig, errors (no I/O)
├─ analyzer-stratum/     # stratum-dsp backend (default)
├─ analyzer/             # Thin facade: re-exports core + default backend helpers
└─ (future) analyzer-oximedia/
```

Follows the same pattern as `library-core` / `library` / `library-adapters`: stable types in `-core`, backends in separate crates.

### 6.2 Dependency rules

| Crate | May depend on | Must not depend on |
|-------|---------------|-------------------|
| `analyzer-core` | `serde`, `thiserror` | `codec`, `symphonia`, `stratum-dsp`, `library`, `engine-dsp` |
| `analyzer-stratum` | `analyzer-core`, `stratum-dsp` | `library`, `engine-dsp` |
| `analyzer` | `analyzer-core`, `analyzer-stratum`, `codec` | `engine-dsp` |
| `library` | `analyzer` (optional feature `analysis`) | direct `stratum-dsp` |

`analyzer` owns the **decode → analyze** orchestration (uses existing `codec` to decode file → mono `f32`). Backends receive PCM only.

### 6.3 Threading model

- Analysis runs **off the audio callback thread** — typically a thread pool or `std::thread` per batch job.
- No locks shared with `engine-dsp` producer.
- Default: **one track at a time** per `LibraryManager` analyze call; batch API may use `rayon` internally (backend already uses rayon).

---

## 7 — Analysis Pipeline

```text
Path / bytes
    │
    ▼
codec::decode_file(path, DecodeOptions { target: MonoF32, max_duration: config.max_duration })
    │
    ▼
Preprocess (analyzer-core policy)
    · peak normalize if |sample| > 1.0
    · optional trim silence (Phase 2)
    · resample to backend preferred rate if needed (stratum-dsp: use native rate when possible)
    │
    ▼
AudioAnalyzer::analyze_pcm(&samples, sample_rate, &AnalysisConfig)
    │
    ▼
TrackAnalysis (backend-neutral)
```

### Decode options (MVP)

- **Channels:** downmix to mono (equal power or simple average — document choice; stratum-dsp expects mono).
- **Sample format:** `f32` normalized ±1.0.
- **Duration cap:** configurable (default: full track; tests use 30–60 s clips).
- **Sample rate:** pass through native rate when supported; otherwise resample via existing `resampler` crate to 44.1 kHz (stratum-dsp default assumption).

---

## 8 — Core Types & API

### 8.1 Error type

```rust
#[derive(Debug, thiserror::Error)]
pub enum AnalyzerError {
    #[error("decode failed: {0}")]
    Decode(String),
    #[error("analysis failed: {0}")]
    Analysis(String),
    #[error("unsupported: {0}")]
    Unsupported(&'static str),
    #[error("backend {backend}: {message}")]
    Backend { backend: &'static str, message: String },
}
```

### 8.2 Configuration

```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AnalysisConfig {
    /// Request BPM/key/grid, or subset.
    pub targets: AnalysisTargets,
    /// Minimum BPM confidence to prefer analysis over file tags.
    pub min_bpm_confidence: f32,
    /// Minimum key confidence to prefer analysis over file tags.
    pub min_key_confidence: f32,
    /// Max milliseconds of audio to decode (None = full file).
    pub max_duration_ms: Option<i32>,
    /// Preferred analysis sample rate (None = native or backend default).
    pub sample_rate: Option<u32>,
}

bitflags::bitflags! {
    pub struct AnalysisTargets: u8 {
        const BPM       = 0b001;
        const KEY       = 0b010;
        const BEAT_GRID = 0b100;
    }
}
```

Default: all targets enabled; `min_*_confidence = 0.5`.

### 8.3 Result types (backend-neutral)

```rust
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct TrackAnalysis {
    pub bpm: Option<BpmAnalysis>,
    pub key: Option<KeyAnalysis>,
    pub beat_grid: Option<BeatGridAnalysis>,
    pub metadata: AnalysisRunMetadata,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct BpmAnalysis {
    pub bpm: f64,
    pub confidence: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct KeyAnalysis {
    /// Musical key, e.g. `"F#m"` or `"Bb major"`. Canonical form for library storage.
    pub musical: String,
    pub confidence: f32,
    pub clarity: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct BeatGridAnalysis {
    /// Beat positions in seconds from start.
    pub beats: Vec<f32>,
    /// Bar (measure) start positions in seconds.
    pub bars: Vec<f32>,
    /// Downbeat positions in seconds.
    pub downbeats: Vec<f32>,
    pub grid_stability: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct AnalysisRunMetadata {
    pub backend: String,
    pub backend_version: String,
    pub analyzed_at: String, // RFC3339
    pub sample_rate: u32,
    pub duration_analyzed_ms: i32,
}
```

Mapping from [stratum-dsp `AnalysisResult`](https://docs.rs/stratum-dsp/latest/stratum_dsp/analysis/result/struct.AnalysisResult.html):

- `bpm` → `BpmAnalysis { bpm, confidence: bpm_confidence }`
- `key` → `KeyAnalysis { musical, confidence: key_confidence, clarity: key_clarity }` — adapters map stratum-dsp `Key` to `musical` only (drop Camelot before library persist)
- `beat_grid` → `BeatGridAnalysis` (direct field copy)
- `grid_stability` → `BeatGridAnalysis.grid_stability`

### 8.4 Trait boundary

```rust
/// Offline analyzer backend (PCM in → analysis out).
pub trait AudioAnalyzer: Send + Sync {
    fn name(&self) -> &'static str;

    fn analyze_pcm(
        &self,
        samples: &[f32],
        sample_rate: u32,
        config: &AnalysisConfig,
    ) -> Result<TrackAnalysis, AnalyzerError>;
}
```

### 8.5 File-level entry point (`analyzer` crate)

```rust
/// Decode a file and run analysis with the default backend.
pub fn analyze_file(
    path: &Path,
    config: &AnalysisConfig,
) -> Result<TrackAnalysis, AnalyzerError>;

/// Analyze already-decoded mono PCM (for tests and custom loaders).
pub fn analyze_pcm(
    samples: &[f32],
    sample_rate: u32,
    config: &AnalysisConfig,
) -> Result<TrackAnalysis, AnalyzerError>;
```

### 8.6 stratum-dsp adapter sketch

```rust
pub struct StratumAnalyzer {
    inner: stratum_dsp::AnalysisConfig,
}

impl AudioAnalyzer for StratumAnalyzer {
    fn name(&self) -> &'static str { "stratum" }

    fn analyze_pcm(&self, samples: &[f32], sample_rate: u32, config: &AnalysisConfig) -> Result<TrackAnalysis, AnalyzerError> {
        let result = stratum_dsp::analyze_audio(samples, sample_rate, self.inner.clone())
            .map_err(|e| AnalyzerError::Backend { backend: "stratum", message: e.to_string() })?;
        Ok(map_stratum_result(result, config))
    }
}
```

Feature flag `ml` on `analyzer-stratum` enables stratum-dsp’s ONNX path and documents the extra `ort` dependency.

---

## 9 — Library Integration

### 9.1 `analyze_track` behavior (updated)

```rust
fn analyze_track(
    &mut self,
    id: &TrackId,
    options: AnalyzeTrackOptions,
) -> Result<AudioSource>;
```

```rust
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnalyzeTrackOptions {
    /// When true, DSP analysis overrides BPM/key from file tags.
    pub force: bool,
}
```

**Steps:**

1. Resolve `AudioSource::File` (streams return `Unsupported` — unchanged).
2. Read tags via `tags::read_tags` → baseline `TrackMetadata`.
3. Run `analyzer::analyze_file(path, &library_analysis_config)`.
4. **Merge policy** (via `merge_analyzed_metadata`):
   - **`force: false` (default):** keep tag BPM/key when present; use analysis only for missing fields (respecting confidence thresholds once implemented).
   - **`force: true`:** always use analysis BPM/key (musical notation) over tag values; beat grid always taken from analysis when `BEAT_GRID` target enabled.
5. Upsert `tracks` row (`key` column = musical string only) + `track_analysis` / beat grid storage (§11).
6. Return updated `AudioSource`.

### 9.4 Key notation policy (library)

**Rule:** `TrackMetadata.key` and `tracks.key` always use **musical notation**. Never store or return Camelot / Open Key codes (`8A`, `12B`, etc.) from library APIs.

| Source | Handling |
|--------|----------|
| File tags (`InitialKey`) | Normalize to musical on read when value looks like Camelot; otherwise pass through if already musical |
| Analyzer backend | Map backend result to `KeyAnalysis.musical` in the adapter (stratum-dsp: use musical name field, not DJ code) |
| `get_track` / `get_collection_tracks` | Return `metadata.key` as musical only |
| Adapters (Rekordbox, Serato, …) | Convert vendor Camelot or vendor-specific codes to musical on import; convert back only on export if the target format requires it |

**Examples of stored values:** `Am`, `F#m`, `Db`, `G major`, `Bb minor` — pick one compact convention in implementation (recommend `"F#m"` / `"Bb"` style for minors/majors without the word *major*).

UI layers that show Camelot wheels compute the mapping client-side from the musical key; the library remains notation-agnostic for DJ wheel display.

### 9.2 Sync policy

During `sync_collection` on folders:

- **Default (MVP):** tag read only (current behavior) — fast import.
- **Optional config** `LibraryConfig.analyze_on_import: bool` — run full analysis on new files (slower scan).

### 9.3 Cargo feature

```toml
# library/Cargo.toml
[features]
default = ["analysis"]
analysis = ["dep:analyzer"]
```

Allows headless tests without linking stratum-dsp when disabled.

---

## 10 — Engine / Playback Integration

Phase 1: **no engine changes required.** Persisted grid is consumed by UI / library adapters later.

Phase 2+ (engine-core / engine-dsp):

- Load beat grid when `Engine::load_track` receives a source that carries analysis (extend `FileAudioSource` or sidecar metadata).
- `Deck` uses persisted grid for **quantized seek**, **sync phase**, and **beat-aligned nudge** — BPM/key from `TrackMetadata`, not live estimation.

---

## 11 — Storage Schema (Library DB)

MVP extends SQLite in `library` (implementation detail — not exposed in `library-core` traits).

### 11.1 Existing columns (unchanged)

`tracks.bpm`, `tracks.key` — `key` is **musical notation only** (see §9.4), populated from merged tag + analysis policy.

### 11.2 New table: `track_analysis`

```sql
CREATE TABLE IF NOT EXISTS track_analysis (
    track_id TEXT PRIMARY KEY NOT NULL REFERENCES tracks(id) ON DELETE CASCADE,
    backend TEXT NOT NULL,
    backend_version TEXT NOT NULL,
    analyzed_at TEXT NOT NULL,
    bpm REAL,
    bpm_confidence REAL,
    key TEXT,
    key_confidence REAL,
    key_clarity REAL,
    grid_stability REAL,
    sample_rate INTEGER NOT NULL,
    duration_analyzed_ms INTEGER NOT NULL
);
```

### 11.3 Beat grid storage

**Option A (MVP):** JSON blob column on `track_analysis`:

```sql
beat_grid_json TEXT  -- {"beats":[...],"bars":[...],"downbeats":[...]}
```

**Option B (Phase 2):** normalized `track_beats (track_id, kind, index, time_secs)` for partial queries.

Start with **Option A** for simplicity; migrate if grid editing/hotcue sync needs indexed beats.

### 11.4 library-core exposure

Phase 1: no new public types required — `get_track` returns `TrackMetadata` with BPM/key.

Phase 2: optional `Library::get_track_analysis(&TrackId) -> Result<Option<TrackAnalysis>>` in `library-core` when UI/engine needs grid without re-analyzing.

### 11.5 Waveform storage (Phase 2+)

**Persist overview only** in `library.db` (`track_waveform` table). See **[`dj-waveform-spec.md`](dj-waveform-spec.md) §8.4, §9, §13** for progressive UI and open decisions.

- **DB:** full-track overview on `analyze_track`.
- **Runtime:** hi-res analysis for the **visible scroll window** only.
- **No** full-track scroll blobs on disk.

---

## 12 — Performance & Resource Limits

| Metric | Target (MVP) |
|--------|----------------|
| 5 min stereo MP3/FLAC | Analyze in < 15 s on typical Linux x86_64 dev CPU (release build) |
| Memory | Peak < 500 MB per concurrent analysis job |
| CI tests | Use ≤ 5 s synthetic WAV; total analyzer test suite < 30 s |
| Parallel batch | Library may analyze sequentially; document `rayon` for future batch CLI |

Mitigations:

- `max_duration_ms` for quick scan mode (e.g. first 90_000 ms only).
- Release profile (`LTO`) for analyzer benchmarks.
- Do not run analysis on audio callback or producer thread.

---

## 13 — Testing & Acceptance Criteria

### Unit tests (`analyzer-core`)

- Serde round-trip for `TrackAnalysis`.
- Merge policy helpers (tag vs analysis confidence).

### Integration tests (`analyzer-stratum`)

- Synthetic click track at known BPM (120, 128) — BPM within ±1.
- Golden-file test: short fixture from `samples/` — snapshot BPM/key/grid ranges (not exact MIK match).
- Empty/silent buffer → graceful error or zero confidence.

### Library tests

- `analyze_track` updates DB `bpm`/`key` when tags absent.
- `analyze_track` preserves high-confidence tags when analysis confidence low (mock backend in tests).

### Acceptance (Phase 1 done when)

- [ ] `cargo test -p analyzer-core -p analyzer-stratum -p analyzer` passes on Linux CI.
- [ ] `library` with `analysis` feature: `analyze_track` persists BPM, key, and beat grid JSON.
- [ ] Documented backend choice and confidence merge policy.
- [ ] No new dependencies on audio callback / producer code paths.

### Benchmarks (criterion, optional CI)

- `analyze_pcm` on 60 s / 44.1 kHz mono fixture.
- Full `analyze_file` including decode.

---

## 14 — Roadmap (Phases)

### Phase 1 — MVP (recommended next sprint)

1. Add `analyzer-core`, `analyzer-stratum`, `analyzer` crates.
2. Implement stratum-dsp adapter + `analyze_file` via `codec`.
3. Wire `library::analyze_track` behind `analysis` feature.
4. Add `track_analysis` table + beat grid JSON.
5. Tests + criterion bench; `.cursor/rules` for analyzer crates.

### Phase 2 — Quality & ops

1. Enable stratum-dsp `ml` feature behind `analyzer-stratum/ml`.
2. Batch analyze API + progress callback.
3. `LibraryConfig.analyze_on_import`.
4. Optional tag write-back (lofty) for BPM/key.
5. Internal benchmark suite vs tagged DJ fixtures (compare to MIK/Rekordbox exports).

### Phase 3 — Playback & extended MIR

1. Expose `TrackAnalysis` on `Library` trait; load grid in engine/deck.
2. Evaluate `analyzer-oximedia` for downbeat/structure if needed.
3. Waveform overview generation (separate module).
4. WASM feasibility study (likely separate lightweight backend).

---

## 15 — Design Decisions (Summary)

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Default backend | stratum-dsp 1.x | Pure Rust, DJ-focused, beat grid, serde; map key to musical for library |
| Key in library | Musical notation only | Single canonical form in DB and `TrackMetadata`; Camelot is UI/adapter concern |
| Crate split | `analyzer-core` + `analyzer-stratum` + `analyzer` | Stable boundary; swap backends without touching library |
| Decode location | `analyzer` crate uses `codec` | Single decode policy; stratum-dsp does not own file I/O |
| Playback tempo/key source | Library track metadata | Single source of truth from offline analysis; no live BPM in `engine-dsp` |
| Tag vs analysis merge | Confidence-based | Respect good vendor tags; override stale/missing |
| Beat grid storage | JSON on `track_analysis` (MVP) | Simple; migrate to normalized beats if needed |
| oximedia-mir | Phase 2 optional | Broader MIR; less DJ validation; use if downbeat/structure gaps |
| FFI backends (aubio/Essentia) | Not MVP | CI complexity, licensing, build weight |

---

## References

- [stratum-dsp docs](https://docs.rs/stratum-dsp/latest/stratum_dsp/) — BPM, key, beat grid, `analyze_audio`
- [stratum-dsp crate](https://crates.io/crates/stratum-dsp) — benchmarks and feature flags
- [oximedia-mir](https://crates.io/crates/oximedia-mir) — optional extended MIR backend
- Project [`tech-spec.md`](tech-spec.md) §10 — library manager and `analyze_track`
