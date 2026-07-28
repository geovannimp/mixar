# Bus hot-cue trigger + saved-loop recall

Date: 2026-07-27  
Parent: [engine event bus](./2026-07-26-engine-event-bus-design.md)  
Status: accepted (implementing)

## Goal

Migrate **trigger hot cue** and **recall saved loop** onto the cmd bus. Save/delete stay on Tauri invokes (library persistence).

## Design

FE already holds `hot_cues` / `saved_loops` in Zustand. Commands carry the media times; engine does not own cue slot tables yet.

| Kind | CmdBody | Engine |
|------|---------|--------|
| `trigger_hot_cue` | `{ position_secs }` | snap (quantize/BPM) → seek → play |
| `recall_saved_loop` | `{ in_secs, out_secs }` | set loop region → seek to in → play |

Empty slot / missing cue: FE no-ops or reports error before publish (same as today’s “empty” errors when possible).

## Out of scope

- `save_hot_cue` / `delete_hot_cue` / `save_loop` / `delete_loop` (library)
- Sampler trigger/assign/banks
- Cue metadata (#98)
