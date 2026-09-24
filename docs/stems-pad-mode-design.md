# Stems pad mode — design (#46)

**Date:** 2026-09-20  
**Status:** Approved for implementation (session gate skipped per owner request)  
**Issue:** [geovannimp/mixar#46](https://github.com/geovannimp/mixar/issues/46)

## Goal (this pass)

Settings-gated offline stem separation → cache under app support → non-blocking deck load → **Stems** pad mode (mute / isolate). No Stem EQ, no Storage UI, no realtime separation in product code.

## Decisions

| Topic | Choice |
|-------|--------|
| Stem set | 4: `vocals`, `drums`, `bass`, `other` (Demucs / issue comment) |
| Feature gate | `AppSettings.stems_enabled` (default **false**) |
| Triggers | When enabled: library analyze **and** deck prepare/load enqueue stem ensure; never block playback |
| Pads | Enabled only when stems ready for the loaded track |
| Pad map | UI pads **1–8** = engine slots **0–7**: mute slots 0–3 (pads 1–4), isolate slots 4–7 (pads 5–8) |
| Inference | **pykeio/ort** + Mixxx HTDemucs ONNX via Mixar `ensure_model` / chunk overlap-add; EP cascade (prefer GPU → CPU) — `docs/stems-ort-onnx-design.md` |
| Input | Interleaved stereo `f32` PCM Mixar already decoded (no second file decode for separation) |
| Stem files | Single NI `.stem.mp4` under `{app_support}/stems/` — `docs/ni-stem-mp4-cache-design.md` |
| Model weights | Mixar `{app_support}/models/` ONNX download/verify; Mixar owns **stem audio** paths in DB — see `docs/stems-storage-models-design.md` |
| Realtime | Document only (see below) |
| charon-audio | **Not a product dependency** (see findings) |

## Finding: charon-audio is scaffolding

We evaluated [charon-audio](https://docs.rs/charon-audio/latest/charon_audio/) because of `Separator::separate(&AudioBuffer)`, Candle, and realtime docs.

Inspecting crate `0.1.0` source:

- `OnnxModel::infer` **returns `vec![input.clone(); num_sources]`** — placeholder, not Demucs.
- Candle path loads safetensors into a `VarMap` but does **not** run an HTDemucs graph; output handling is incomplete.
- `ModelZoo::download_model` is documented as a placeholder.
- Published “6× Python Demucs” benchmarks are not backed by a real separator in this crate version.
- `RealtimeSeparator` is a CPAL **input** stream that calls the same stub `infer` — useful as a **pattern** for a future Mixar realtime path, not shippable quality.

**Keep as reference:** AudioBuffer-shaped PCM API, segment/overlap processor shape, realtime buffer sketch.  
**Do not depend on charon for separation quality.**

Burn HTDemucs was evaluated and dropped for speed; shipping path is ORT + Mixxx HTDemucs ONNX (supersedes Burn / StemSplit notes). Out of this pass: settings model picker / FT bag.

## Architecture

```text
Settings.stems_enabled
        │
        ▼
library analyze / deck prepare
        │  enqueue if missing/stale
        ▼
analyzer-stems
  ensure_model (Mixar models/ + Mixxx HTDemucs ONNX)
  ort Session + EP cascade → overlap-add chunks
  write 4 Opus/FLAC + DB rows
        │
        ├── Deck loads original ASAP
        └── When ready: engine attaches 4 stem buffers
                └── PadMode::Stems → mute / isolate gains
```

### Crates

| Crate | Responsibility |
|-------|----------------|
| `analyzer-stems` (new) | PCM → 4 stems; model ensure; write Opus/FLAC; progress callbacks |
| `library` | `track_stem` metadata; `ensure_track_stems`; worker enqueue; events |
| `engine-api` / `engine-core` / `engine-dsp` | `PadMode::Stems`; stem attach; per-stem gains; pad handlers |
| `host-flutter` + Flutter | Settings toggle; Stems pad UI; FRB |

`engine-dsp` stays pure (no I/O): receives `Arc<LoadedAudio>` per stem like the main deck buffer.

### Cache / DB

**Filesystem**

- `{app_support}/stems/{fnv64(track_id)}/vocals.{opus|flac}` (and drums/bass/other)
- Model weights: `{app_support}/models/{name}-{sha8}.onnx`

**Table `track_stem`** (one row per track when complete)

- `track_id` PK
- `backend` (e.g. `htdemucs_mixxx_v1/{ep}`) — model id before `/` compared to current `DEFAULT_MODEL` before cache reuse
- `source_fingerprint` (path mtime/size + PCM shape) — stale when source changes
- `sample_rate`
- `generated_at`
- absolute paths for the four stems

Stale if model/backend changes, fingerprint mismatches, or files missing.

### Playback

- Until stems ready: play original `LoadedAudio` only; Stems pads disabled / inert.
- When ready: `Deck` keeps original for fallback unload, but **audible path** sums four stem buffers at the **same** `position_frac` / tempo / loop / slip / key-lock path.
- Effective gain per stem: `0` if muted; if isolate set, only that stem is `1` (others `0`); else unmuted stems `1`.
- Session-only mute/isolate (not persisted). Snapshot fields for UI.

### Pad mode

Extend `PadMode` with `Stems`.

User-facing pad labels are **1–8**; the engine and controller use **zero-based slots 0–7**.
Stem indices follow NI order: **drums, bass, other, vocals**.

Pad chrome matches Jump’s two-line layout: action on line 1, stem name on line 2 (`mute` / `solo` + `drums`/`bass`/`other`/`vocal`).

| Engine slot | UI pad | Action |
|-------------|--------|--------|
| 0–3 | 1–4 | Toggle mute drums / bass / other / vocals |
| 4–7 | 5–8 | Set isolate to that stem (press again / same slot clears isolate) |

| Slot | Action | Label line 1 | Label line 2 |
|------|--------|--------------|--------------|
| 0 | Mute drums | mute | drums |
| 1 | Mute bass | mute | bass |
| 2 | Mute other | mute | other |
| 3 | Mute vocals | mute | vocal |
| 4 | Isolate drums | solo | drums |
| 5 | Isolate bass | solo | bass |
| 6 | Isolate other | solo | other |
| 7 | Isolate vocals | solo | vocal |

Controller MIDI: extend pad_mode mapping like Sampler.

Progress / analyze vs stems jobs: `docs/stems-analyze-progress-design.md`.

### Settings / analyze / load

- `stems_enabled: bool` on `AppSettings` (default false), Settings → Library (or Analysis) panel.
- Analyze path: after BPM/waveform (or parallel worker job), if enabled → `ensure_track_stems`.
- Deck prepare/load: start playback with prepared original; if enabled and stems missing, spawn ensure **without** holding host session lock; on completion emit library/engine event so UI enables pads and engine can `attach_stems`.

### Realtime (documentation only)

**charon `RealtimeSeparator` pattern (for a future pass):**

1. Own a model + processor behind `Arc<Mutex<_>>`.
2. Accumulate interleaved input until `buffer_size * channels`.
3. Reshape to planar `[channels, frames]`, run `infer`, publish per-source output buffers.
4. Consumer reads `get_output(source_index)`.

**Mixar adaptation notes (when we build it):**

- Do **not** open a second CPAL input device — feed from the deck’s already-playing PCM (or a sidechain ring) so it stays in sync with transport.
- Only useful as fallback while offline stems generate; quality/latency will be worse than cached stems.
- Requires a **real** windowed Demucs path (ORT + Mixxx HTDemucs ONNX), not charon’s stub infer.
- Keep realtime off the audio callback’s critical path: hop-sized jobs on a worker, crossfade results.

## Out of scope

- Stem EQ (VirtualDJ HI/MID/LOW)
- Settings → Storage stem cleanup UI
- Stems FX pad mode
- 5-stem Kick/HiHat split
- Browser/mobile-specific ORT packaging
- Settings model picker (FT / MDX)

## Testing

- `analyzer-stems`: unit test windowing + mock/fixture separator OR short synthetic PCM through public API when model available; cache hit skips re-infer.
- `library`: ensure stems writes rows; disabled setting skips.
- `engine-dsp`: summing four buffers with mute/isolate gains at shared playhead.
- Flutter: pad mode tab + disabled until ready (widget/unit).

## Success criteria

1. With stems disabled, behavior unchanged.
2. With stems enabled, analyze or load eventually produces four stem files + DB row without blocking first audio.
3. Stems pad mode mutes/isolates layers when ready.
4. Design notes capture charon realtime for later.
