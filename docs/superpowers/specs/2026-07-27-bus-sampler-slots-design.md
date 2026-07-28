# Bus sampler slots + bank select (host-handled)

Date: 2026-07-27  
Parent: [engine event bus](./2026-07-26-engine-event-bus-design.md)  
Status: accepted (implementing)

## Goal

Move `assign_sampler_slot`, `assign_sampler_slot_from_track`, `clear_sampler_slot`, and `set_deck_sampler_bank` off per-action Tauri invokes onto `engine_publish` / `publishCmd`.

## Design

These cmds are **host-handled**: `bus_bridge` runs the existing AppState/library/engine-assign logic and emits legacy `engine://event` status. They are **not** forwarded to the engine omnibus (library persistence cannot live on the control thread).

| Kind | Body | Host |
|------|------|------|
| `assign_sampler` | `{ slot, path }` | same as today’s path assign |
| `assign_sampler_track` | `{ slot, track_id }` | same as today’s track assign |
| `clear_sampler` | `{ slot }` | same as today’s clear |
| `set_sampler_bank` | `{ bank_id }` | same as today’s set bank |

Origin: `Deck(deck_id)`.

## Out of scope

- Bank create/update/delete (next slice)
- Sampler trigger/end (already on engine bus)
