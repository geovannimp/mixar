# Stems storage + Mixar model cache — design

**Date:** 2026-09-21  
**Status:** Approved (owner: skip per-section review; implement)  
**Issue:** [geovannimp/mixar#46](https://github.com/geovannimp/mixar/issues/46)  
**Depends on:** stems pad mode foundation (`docs/stems-pad-mode-design.md`, #380)

## Goal

1. Own HTDemucs ONNX weights under Mixar app-support (not stem-splitter-core ProjectDirs).
2. Minimal Settings → Storage UI: stem-cache size, model size, clear-all for each.

## Decisions

| Topic | Choice |
|-------|--------|
| Storage UI depth | Minimal: sizes + Clear all stems / Clear model |
| Model location | `{app_support}/models/` — Mixar download + verify + `ModelHandle` |
| SSC ProjectDirs | Ignore (no migrate; may re-download once for early testers) |
| Clear stems | Delete `{stems_root}/**` and all `track_stem` rows; never touch original audio |
| Clear model | Delete `{models_root}/**` |
| Eviction | Manual only |

## Architecture

```text
{app_support}/
  library.db
  settings.json
  stems/{fnv64(track_id)}/*.wav
  models/{name}-{sha8}.onnx          ← NEW

analyzer-stems::ensure_model(models_root, name)
  → registry manifest + download/verify into models_root
  → ModelHandle { manifest, local_path } → engine::preload

LibraryBuses: stems_root + models_root
LibraryTransport: storageUsage / clearStemCache / clearModelCache

Settings → Storage
  Stem cache: N  [Clear]
  Model cache: N [Clear]
```

## Components

| Piece | Change |
|-------|--------|
| `analyzer-stems` | `ensure_model(models_root, …)`; split takes `models_root`; no SSC `ensure_model` default cache |
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
- Deleting leftover SSC ProjectDirs caches

## Testing

- `analyzer-stems`: ensure writes under temp `models_root` only.
- Library/host: clear stems removes files + rows; clear models empties dir.
- Flutter: Storage section builds; clear triggers host (widget/smoke if cheap).

## Success criteria

1. Stem separation uses Mixar `{app_support}/models/` exclusively for weights.
2. Storage shows stem + model sizes and can clear each without deleting music files.
3. After clear, regenerate / re-download works when stems are enabled.
