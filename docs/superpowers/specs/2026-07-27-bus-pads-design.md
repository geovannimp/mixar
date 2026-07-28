# Bus pads slice (pad mode + loop roll)

Date: 2026-07-27  
Parent: [engine event bus](./2026-07-26-engine-event-bus-design.md)  
Status: draft

## Goal

Migrate deck **pad mode** and **loop roll** from per-action Tauri invokes onto the engine cmd/evt bus, matching the channel-strip / sync / performance slices.

## Current state

| Concern | Today |
|--------|--------|
| `pad_mode` | Tauri `AppState` only (`set_deck_pad_mode`); FE gets it via legacy deck/status publish |
| Loop roll | Tauri stashes `loop_roll_restore`, calls `Engine::set_deck_loop_region` / `clear_deck_loop` |
| Snapshot | Engine already emits `active_loop`, `quantize`; **no** `pad_mode` |

## Design

### Engine owns pad UI mode and roll restore

Move into `engine-core` per-deck control state (alongside `sync_mode` / `quantize`):

- `pad_mode: PadMode` (default `hot_cue`)
- `loop_roll_restore: Option<LoopRegion>` — previous active loop when a roll starts

### Wire / API (`engine-api`)

**New `Kind`s:** `SetPadMode`, `BeginLoopRoll`, `EndLoopRoll`

**New type:** `PadMode` — `hot_cue` | `loop_roll` | `beat_jump` | `sampler` (serde snake_case; mirror FE/`deck_sync` today)

**`CmdBody`:**

- `set_pad_mode { mode: PadMode }`
- `begin_loop_roll { beats: u32 }` (`beats >= 1`)
- `end_loop_roll` → empty / dedicated empty-ish variant via existing empty pattern, or tagged with no fields — prefer `EndLoopRoll` with no payload fields (use `Empty` body + kind, same as `exit_loop`)

**Snapshots:** add `pad_mode` to `DeckSnapshot` and `EvtBody::DeckUpdated`.

No new evt kinds; success continues to emit `Updated` with the deck snapshot (including `active_loop` after roll begin/end).

### Engine behavior

**`set_pad_mode`:** store mode; emit `Updated`. No audio side effects in engine.

**`begin_loop_roll(beats)`:**

1. Require BPM (error evt if missing), `beats >= 1`.
2. Stash current `active_loop` into `loop_roll_restore` (clone; may be `None`).
3. Compute in/out like today’s Tauri helper: snap playhead with quantize, length = `beats * (60/bpm)`, clamp to duration.
4. `set_deck_loop_region`; emit `Updated` with active loop.

**`end_loop_roll`:**

1. Take `loop_roll_restore`.
2. If restore was `Some` and `active` → restore that region; else `clear_deck_loop`.
3. Emit `Updated`.

### Host (Tauri)

- Delete `set_deck_pad_mode`, `begin_loop_roll`, `end_loop_roll` commands and their `invoke` handlers registration.
- **`engine_publish` bridge side effects** (same ceiling as Unload metadata clear):
  - On `SetPadMode` for a deck: mirror `AppState.decks[id].pad_mode` (sampler invokes still gate on AppState).
  - On `SetPadMode(Sampler)`: call existing `ensure_deck_bank_loaded` (library still host-owned).
- Stop relying on `publish_deck` return values for these three actions; FE applies bus events only.

### Frontend

- `setDeckPadMode` / `beginLoopRoll` / `endLoopRoll` → `publishCmd`.
- Extend wire codecs + `CmdKind` + `applyBusEvent` / snapshot mapping for `pad_mode`.
- Leave hot-cue / saved-loop / sampler trigger invokes unchanged.

### Tests

Headless `crates/engine-core/tests/bus_pads.rs`:

1. `set_pad_mode` → `Updated` carries new `pad_mode`.
2. `begin_loop_roll` with BPM → `active_loop` set; `end_loop_roll` clears when no prior loop.
3. Prior active loop → begin roll → end roll restores prior region.

## Out of scope

- Hot cue / saved loop persistence and trigger
- Sampler play/assign/bank CRUD (beyond the SetPadMode→bank-load bridge hook)
- Track load / `start_engine`
- Optional position timestamp on loop roll (see #97 for loop in/out; roll can follow later)

## Non-goals / ceilings

- `ponytail:` AppState still mirrors `pad_mode` for leftover sampler invokes until the sampler bus slice; bridge update is intentional duplication, not a second source of truth for the UI (Zustand follows evt bus).
