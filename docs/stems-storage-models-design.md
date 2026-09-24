# Stems storage + Mixar model cache — design

**Date:** 2026-09-21  
**Status:** Approved (owner: skip per-section review; implement)  
**Issue:** [geovannimp/mixar#46](https://github.com/geovannimp/mixar/issues/46)  
**Depends on:** stems pad mode foundation (`docs/stems-pad-mode-design.md`, #380)  
**Inference:** ORT + Mixxx HTDemucs ONNX — `docs/stems-ort-onnx-design.md`  
**Cache format:** NI `.stem.mp4` — `docs/ni-stem-mp4-cache-design.md`

## Goal

1. Own HTDemucs ONNX weights under Mixar app-support (not third-party ProjectDirs caches).
2. Minimal Settings → Storage UI: stem-cache size, model size, clear-all for each.

## Decisions

| Topic | Choice |
|-------|--------|
| Storage UI depth | Minimal: sizes + Clear all stems / Clear model |
| Model location | `{app_support}/models/` — Mixar download + verify + ONNX path for ort Session |
| Clear stems | Delete `{stems_root}/**` and all `track_stem` rows; never touch original audio |
| Sync stems | Remove orphan files under `{stems_root}` not referenced by `track_stem`, and drop rows whose files are missing |
| Clear model | Delete `{models_root}/**` |
| Eviction | Manual only |
| Cache backend id | `{model}/{ep}`, default `htdemucs_mixxx_v1/<ep>` (older SSC / Burn ids miss and regenerate) |

## Architecture

```text
{app_support}/
  library.db
  settings.json
  stems/{fnv64(track_id)}.stem.mp4   ← NI Stem cache (see docs/ni-stem-mp4-cache-design.md)
  models/{name}-{sha8}.onnx          ← HTDemucs ONNX (Mixxx `htdemucs_mixxx_v1`)

analyzer-stems::ensure_model(models_root, name)
  → registry manifest + download/verify into models_root
  → local ONNX path → ort Session (EP cascade) → mux `.stem.mp4`

LibraryBuses: stems_root + models_root
LibraryTransport: storageUsage / clearStemCache / clearModelCache

Settings → Storage
  Stem cache: N  [Clear]
  Model cache: N [Clear]
```

## Components

| Piece | Change |
|-------|--------|
| `analyzer-stems` | `ensure_model(models_root, …)`; split takes `models_root`; ort infer + `.stem.mp4` mux |
| `library` | `models_root` on buses; `clear_all_track_stems` (files + rows); dir size helpers |
| `host-flutter` | Set `models_root` next to `stems` on open; FRB storage usage/clear |
| Flutter | `SettingsSection.storage` + panel; confirm dialogs; refresh sizes |

## Error handling

- Clear while a deck holds attached stems: OK — files may be open on some OSes; best-effort delete, report partial failure via toast. Next load re-ensures.
- Model missing after clear: next ensure re-downloads into Mixar `models/`.
- Usage scan: recursive byte sum; treat missing root as 0.

## Out of scope

- Per-track / playlist cleanup
- Automatic eviction
- Stem EQ / realtime
- Automatic migration of leftover third-party / Burn model caches

## Testing

- `analyzer-stems`: ensure writes under temp `models_root` only.
- Library/host: clear stems removes files + rows; clear models empties dir.
- Flutter: Storage section builds; clear triggers host (widget/smoke if cheap).

## Success criteria

1. Stem separation uses Mixar `{app_support}/models/` exclusively for weights.
2. Storage shows stem + model sizes and can clear each without deleting music files.
3. After clear, regenerate / re-download works when stems are enabled.
