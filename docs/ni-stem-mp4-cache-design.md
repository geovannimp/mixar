# NI `.stem.mp4` cache & playback — design

**Date:** 2026-09-23  
**Status:** Approved (brainstorm)  
**Depends on:** stems pad mode (`docs/stems-pad-mode-design.md`), storage/models (`docs/stems-storage-models-design.md`), ORT HTDemucs (`docs/superpowers/specs/2026-09-22-stems-ort-onnx-design.md`)  
**References:** [NI Stems](https://www.native-instruments.com/pages/stems), [Mixxx stem mixing](https://mixxx.org/news/2024-08-26-stem-mixing/), [stemgen `nistem.rs`](https://github.com/acolombier/stemgen/main/src/nistem.rs)

## Goal

Replace Mixar’s four-file Opus/FLAC stem cache with a single NI-compatible `.stem.mp4`. Support opening user Stem files and Mixar-generated cache through one load path. When a valid Stem is available for a track, that file is the playback source (mixdown + four stems) instead of the original audio.

## Decisions

| Topic | Choice |
|-------|--------|
| Cache format | Single `{stems_root}/{fnv64(track_id)}.stem.mp4` |
| User Stem files | Open in place; never Demucs; never write Mixar cache |
| When stems are prepared | Early only (analyze / library worker). **No** ensure or hot-swap on deck load |
| Deck load if cache missing | Play original only; Stems pads inert |
| Mux mode | **Consistent streams** — all five streams same codec + sample rate |
| Codecs | Settings `stems_format`: `opus` (default) \| `aac` \| `flac`. Each option must encode **and** decode in pure Rust inside `codec`; if AAC encode is not available in-tree yet, keep the setting but ship opus/flac first and reject `aac` ensure with a clear error until the encoder lands. |
| Tooling | Pure Rust mux/demux in `codec` (Symphonia for demux/decode; dedicated MP4 writer + STEM atom — **no ffmpeg**) |
| Stem order (file + engine) | NI: `drums`, `bass`, `other`, `vocals` (mixdown = stream 0) |
| UI pad labels | Out of scope for this pass; engine indices follow NI order |
| `mastering_dsp` | Write disabled defaults in STEM atom; do not implement playback |
| Old 4-file cache | Stale; wipe/ignore; no dual-format reader |

## Architecture

```text
Early prep (analyze / library worker, stems_enabled)
  └─ source is normal audio (not a Stem file)
        └─ Demucs → mux consistent .stem.mp4 → track_stem.path

Prepare / load track
        │
        ├─ source is .stem.mp4? ──► StemBundle (in place) ──► deck load
        │
        ├─ stems_enabled && valid cache? ──► StemBundle(cache) ──► deck load
        │         (main = mixdown; stems attached)
        │
        └─ else ──► decode original ──► deck load
              (no ensure, no hot-swap)
```

### `StemBundle`

Conceptual type used by prepare/host:

- `mixdown` — main `LoadedAudio` (or PCM + rate)
- `stems: [LoadedAudio; 4]` — **drums, bass, other, vocals**
- optional STEM atom labels/colors
- `source_path`

Demucs model output may use a different slot order; **permute once** when packing the bundle/file. Load does not remap.

## Components

| Piece | Responsibility |
|-------|----------------|
| `codec` | Detect/open `.stem.mp4`; demux all audio streams; mux consistent-stream `.stem.mp4` + STEM JSON atom. Keep single-track `AudioDecoder` for normal files. |
| `analyzer-stems` | HTDemucs split → PCM stems; call mux helper; drop multifile Opus/FLAC cache writers as the product cache path. |
| `library` | `track_stem` with single `path`; ensure only from analyze/worker; skip ensure for native Stems; clear-cache deletes `.stem.mp4` tree (+ old dirs). |
| Prepare / host | Resolve native Stem **or** cache hit → `StemBundle`; else main-only. Never enqueue stem ensure on deck load. |
| `engine-dsp` | Main buffer + optional four stems; NI index order. Bundle load = mixdown as main + attach stems immediately. |

## File format

- Container: MP4, extension `.stem.mp4`
- Five stereo audio streams, identical codec and sample rate
- Stream 0 = mixdown (re-encoded from source PCM for generated files)
- Streams 1–4 = drums, bass, other, vocals
- STEM atom JSON (`version: 1`, four `{name, color}`, `mastering_dsp` disabled) — labels/colors match stemgen defaults (Drums/Bass/Other/Vocals)
- Detection: `.stem.mp4` extension for browse/import; validate on open (≥5 audio streams + STEM atom preferred)

## Cache & DB

**Filesystem (generated):** `{app_support}/stems/{fnv64(track_id)}.stem.mp4`

**`track_stem` (generated cache only):**

- `track_id` PK
- `path` — absolute path to the `.stem.mp4` (replaces `vocals_path` / `drums_path` / `bass_path` / `other_path`)
- `backend`, `source_fingerprint`, `format`, `sample_rate`, `generated_at`

Native library Stem tracks: **no** `track_stem` row; prepare opens `source_ref` directly.

**Stale when:** format setting changes, fingerprint mismatches, file missing, or legacy four-file directory layout present without a valid `.stem.mp4` row.

**Migration:** `ensure` / `clear_all_track_stems` remove old per-track directories; analyze regenerates when enabled.

## Error handling

- Corrupt cache `.stem.mp4`: drop/invalidate row; fall back to original on prepare.
- Corrupt **native** Stem source: fail loudly (toast / prepare error); do not Demucs.
- Ensure writes to temp then rename; failed temp deleted; no partial DB row on failure.
- Codec inside a user Stem that Symphonia cannot decode: error; no silent fallback to Demucs.
- Clear cache while in use: best-effort delete (unchanged policy).

## Out of scope

- Deck-load ensure / hot-swap
- ffmpeg
- Embedded mastering DSP playback
- UI pad label reorder polish
- Realtime separation / Stem EQ
- Copying generated cache into the user’s music folder

## Testing

- Round-trip: PCM → mux `.stem.mp4` → demux → five streams, NI order, STEM atom present
- Open fixture `.stem.mp4` as `StemBundle`
- Library: ensure writes one file + one `path`; legacy four-file layout ignored; clear wipes `.stem.mp4`
- Prepare resolution: native / cache hit / miss
- Engine: stem indices drums→bass→other→vocals (one assert)

## Success criteria

1. Analyze generates `{key}.stem.mp4`; prepare uses it instead of the original when present.
2. Opening a user `.stem.mp4` plays mixdown + stems with no Demucs.
3. Generated files follow NI stream order + STEM atom (openable elsewhere as Stems).
4. Happy path no longer uses a multifile stem cache.
