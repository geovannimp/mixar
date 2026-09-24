# Analyze / stems split + progress — Design

**Date:** 2026-09-21  
**Status:** Approved  
**Related:** `docs/stems-pad-mode-design.md`, `docs/stems-ort-onnx-design.md`, `docs/ni-stem-mp4-cache-design.md`

## Goal

1. **Split** library analysis (BPM/key/loudness/waveform) from stem ensure so Analyzing… can finish when analysis finishes, while stems continue independently.
2. **Parallelize** analysis DSP and stem work (shared decode when useful; Demucs still single-flight).
3. **Progress feedback** for both jobs (phase + optional fraction) instead of a silent loader.
4. **STEMS GENERATING…** only while a stem job is actually in flight; disk-full / encode failures surface as library errors.

## Decisions

| Topic | Choice |
|-------|--------|
| Analyze vs stems | Separate jobs; `TrackAnalyzed` after analysis persist only |
| Stem failure | Does not undo BPM/key; `EvtBody::Error` with clear message |
| Parallelism | After shared PCM ready: analysis thread + stem ensure thread; existing Demucs mutex stays |
| Progress | `EvtBody::TrackProgress { track_id, phase, fraction }` on library evt bus |
| Pad banner | `stemsGenerating` true only between stem-job start and ready/failed |
| Disk full | Map `ENOSPC` / quota / “No space” → “Stem cache write failed: disk full” (toast via existing error path) |

## Progress phases

| Phase | Job | Notes |
|-------|-----|-------|
| `decode` | shared / either | Loading PCM |
| `bpm` / `key` / `loudness` / `waveform` | analysis | Emit as each step starts (or completes); fraction optional |
| `stems_model` | stems | Model ensure / download |
| `stems_separate` | stems | Demucs windows; `fraction` = windows done / total |
| `stems_encode` | stems | Mux `.stem.mp4`; fraction optional |
| `stems_ready` | stems | Terminal success |
| `stems_failed` | stems | Terminal failure (also emit `Error`) |

UI: library row / detail shows short phase label (+ % when fraction present). Analyzing… clears on `TrackAnalyzed` or analysis `Error` only.

## Pad / engine

- Deck `Updated` authors `stemsReady` and **`stemsGenerating`**.
- Banner iff `stemsGenerating`; pads live iff `stemsReady`; else disabled, no banner (idle / failed / stems off).

## Analyze worker flow (stems enabled)

```text
decode PCM (progress: decode)
        │
        ├── analysis thread → bpm/key/loudness/waveform → persist → TrackAnalyzed
        └── stems thread → model → separate → mux .stem.mp4 → upsert
              ├── success: stems_ready
              └── failure: stems_failed + Error (disk full mapped)
```

When stems disabled: analysis only (unchanged completion).

Stem ensure runs from analyze / library worker only — **not** on deck load (`docs/ni-stem-mp4-cache-design.md`).

## Out of scope

- Smooth fake progress animation
- Stem EQ / realtime separation

## Verification

- Analyze with stems on: Analyzing… clears when BPM/key land; STEMS GENERATING… only while stem job runs; then pads enable.
- Stem ENOSPC: error toast; generating clears; pads stay disabled; analysis row still has BPM.
- Progress events update UI phase text during analyze + stems.
