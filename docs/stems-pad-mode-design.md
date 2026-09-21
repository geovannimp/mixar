# Stems pad mode — design (#46)

**Date:** 2026-09-20  
**Status:** Approved for implementation (session gate skipped per owner request)  
**Issue:** [geovannimp/mixar#46](https://github.com/geovannimp/mixar/issues/46)

## Goal (this pass)

Settings-gated offline stem separation → cache under app support → non-blocking deck load → **Stems** pad mode (mute / isolate). Model weights live under Mixar `{app_support}/models/` (see `docs/stems-storage-models-design.md`). No Stem EQ, no realtime separation in product code.

## Decisions

| Topic | Choice |
|-------|--------|
| Stem set | 4: `vocals`, `drums`, `bass`, `other` (Demucs / issue comment) |
| Feature gate | `AppSettings.stems_enabled` (default **false**) |
| Triggers | When enabled: library analyze **and** deck prepare/load enqueue stem ensure; never block playback |
| Pads | Enabled only when stems ready for the loaded track |
| Pad map | UI pads **1–8** = engine slots **0–7**: mute slots 0–3 (pads 1–4), isolate slots 4–7 (pads 5–8) |
| Inference | **stem-splitter-core** HTDemucs ONNX (`htdemucs_ort_v1`) via Mixar `ensure_model(models_root)` + `preload` / `run_window_demucs` |
| Input | Interleaved stereo `f32` PCM Mixar already decoded (no second file decode for separation) |
| Stem files | WAV under `{app_support}/stems/{fnv64(track_id)}/` |
| Model weights | `{app_support}/models/` (Mixar download/verify; ignore SSC ProjectDirs) — see `docs/stems-storage-models-design.md` |
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

[demucs-rs](https://github.com/nikhilunni/demucs-rs) (Burn) remains the long-term cross-platform candidate if we outgrow ONNX. Out of this pass.

## Architecture

```text
Settings.stems_enabled
        │
        ▼
library analyze / deck prepare
        │  enqueue if missing/stale
        ▼
analyzer-stems
  ensure_model (stem-splitter-core)
  window PCM → run_window_demucs
  write 4 WAVs + DB rows
        │
        ├── Deck loads original ASAP
        └── When ready: engine attaches 4 stem buffers
                └── PadMode::Stems → mute / isolate gains
```

### Crates

| Crate | Responsibility |
|-------|----------------|
| `analyzer-stems` (new) | PCM → 4 stems; model ensure; write WAV; progress callbacks |
| `library` | `track_stem` metadata; `ensure_track_stems`; worker enqueue; events |
| `engine-api` / `engine-core` / `engine-dsp` | `PadMode::Stems`; stem attach; per-stem gains; pad handlers |
| `host-flutter` + Flutter | Settings toggle; Stems pad UI; FRB |

`engine-dsp` stays pure (no I/O): receives `Arc<LoadedAudio>` per stem like the main deck buffer.

### Cache / DB

**Filesystem**

- `{app_support}/stems/{fnv64(track_id)}/vocals.wav` (and drums/bass/other)
- Model weights: Mixar owns the cache under the supplied `models_root` in app support (`{app_support}/models/`); do not use `stem-splitter-core` `ProjectDirs`

**Table `track_stem`** (one row per track when complete)

- `track_id` PK
- `backend` (model id, e.g. `htdemucs_ort_v1`) — compared to current `DEFAULT_MODEL` before cache reuse
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

| Engine slot | UI pad | Action |
|-------------|--------|--------|
| 0–3 | 1–4 | Toggle mute vocals / drums / bass / other |
| 4–7 | 5–8 | Set isolate to that stem (press again / same slot clears isolate) |

Controller MIDI: extend pad_mode mapping like Sampler.

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
- Requires a **real** windowed Demucs path (stem-splitter / Burn), not charon’s stub infer.
- Keep realtime off the audio callback’s critical path: hop-sized jobs on a worker, crossfade results.

## Out of scope

- Stem EQ (VirtualDJ HI/MID/LOW)
- Stems FX pad mode
- 5-stem Kick/HiHat split
- Browser/mobile Burn backend
- Per-track Storage cleanup / auto-eviction (manual clear-all shipped in storage follow-up)

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
