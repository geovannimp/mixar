# Drop Tauri `engine_controller` Design

**Issue:** [#119](https://github.com/geovannimp/rust-dj-engine/issues/119)  
**Date:** 2026-07-31  
**Status:** Implemented  
**Related:** [engine event bus design](2026-07-26-engine-event-bus-design.md)

## Goal

Remove the host parallel publish path that rebuilt `DeckStatus` / `EngineStatus` from mirrored `AppState` and `app.emit`’d MessagePack onto `engine://bus`. Host-enriched egress must use the session **evt bus** so `EvtForwarder` is the only webview emitter.

## Decisions

| Topic | Choice |
|-------|--------|
| Host enrichment | Still host-assembled (metadata, hot cues, loops, sampler); not written into DSP |
| Publish API | `EngineSession::publish_evt(origin, kind, EvtBody)` — encode, bump revision, publish |
| Overlay | Engine `deck_snapshot` / `engine_status_snapshot` + host enrichment fields |
| Helpers | `bus_bridge::publish_deck_updated` / `publish_engine_status` |
| AppState | Host enrichment + session/settings only — no transport/mix/revision mirror |
| UI mirror | Zustand via `engine://bus` only |

## Flow

```text
Host mutation (load / cue / sampler / start)
  → overlay engine snapshot with AppState enrichment
  → session.publish_evt(...)
  → evt bus
  → EvtForwarder.encode_wire → emit("engine://bus")
  → EngineTransport → Zustand
```

Engine-handled cmds (play, EQ, …) already publish slim `Updated` / `Status` from `engine-core` control; FE preserves metadata across slim patches.

## Non-goals

- Library I/O on the control thread
- MIDI consumer
- Full metadata ownership inside DSP snapshots
