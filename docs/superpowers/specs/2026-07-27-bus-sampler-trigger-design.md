# Bus sampler trigger / end

Date: 2026-07-27  
Parent: [engine event bus](./2026-07-26-engine-event-bus-design.md)  
Status: accepted (implementing)

## Goal

Migrate `trigger_sampler_pad` and `end_sampler_pad` onto the cmd bus. Bank assign/CRUD/set-bank stay on Tauri invokes (library).

## Design

| Kind | CmdBody | Engine |
|------|---------|--------|
| `trigger_sampler` | `{ slot: u8 }` | Require `pad_mode == sampler`; `trigger_sampler` |
| `end_sampler` | `{ slot: u8 }` | `end_sampler` |

### Host bridge (before publish)

On `trigger_sampler` only (Unload / SetPadMode pattern):

1. `apply_effective_play_mode`
2. `ensure_deck_bank_loaded`
3. After successful `publish_cmd`, persist track last-used sampler bank (same as today)

Pad-mode gate lives in **engine** (source of truth after pads slice).

## Out of scope

- assign / clear / bank CRUD / set deck bank
- Sampler status snapshot on bus (unchanged; trigger does not mutate slot metadata)
