# Bus load path / library track (host-handled)

Date: 2026-07-27  
Parent: [engine event bus](./2026-07-26-engine-event-bus-design.md)  
Status: accepted

## Goal

`load_path_to_deck` and `load_library_track_to_deck` via `publishCmd` / host-handled `engine_publish`.

| Kind | Origin | Body |
|------|--------|------|
| `load_path` | Deck | `{ path }` |
| `load_library_track` | Deck | `{ track_id }` |

Not forwarded to omnibus. Host runs existing load inners (decode, library resolve, engine load, legacy deck/status publish). `start_engine` stays a Tauri invoke (no session / bus until the engine is up).
