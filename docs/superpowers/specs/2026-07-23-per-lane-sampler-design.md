# Per-lane sampler + strip routing

Date: 2026-07-23  
Status: accepted

## Goal

Move the engine sampler from a shared mixer-level inject into each `MixerLane`, so:

1. Pads belong to a deck in DSP (same mental model as the UI).
2. Two decks can play the same sample at once (independent voice pools).
3. A setting chooses whether sampler audio is mixed **before** or **after** the channel strip (EQ / filter / fader / crossfader path). Default: **before**.

## Industry note

Serato/rekordbox-style samplers often bypass the deck strip by default. We deliberately default to **before** strip because our pads live on the deck UI and “pads follow this deck’s EQ/fader” is less confusing for MVP.

## Current state

- `Mixer` owns one `Sampler` and mixes it into the master buffer after lane summing.
- Engine assign/trigger APIs are global (no `deck_id`).
- App keeps a single `loaded_sampler_bank_id` and loads that bank into the shared sampler.

## Target architecture

```text
MixerLane::begin_render(frames)   # reset strip / PFL for this callback only
each Node::process chunk:
  dry = Deck::process(chunk_frames)
  if route == BeforeStrip:
      sum = dry + Sampler::render(chunk)
      ChannelStrip(sum) → lane output
  else:  # AfterStrip
      ChannelStrip(dry) → lane_out
      lane_out += Sampler::render(chunk)    # post-strip
      → lane output

Mixer sums lane outputs (+ crossfader) → buses
# no mixer-level Sampler
```

### Ownership

| Piece | Owner |
|-------|--------|
| `Sampler` (slots + voices) | `MixerLane` |
| Strip routing mode | Global setting applied to all lanes |
| Loaded bank cache | Per deck (`loaded_sampler_bank_id[deck]` or deck field) |

### Engine API changes

All sampler ops take `deck_id` (lane index):

- `assign_sampler_slot(deck_id, slot, …)`
- `clear_sampler_slot(deck_id, slot)`
- `clear_all_sampler_slots(deck_id)`
- `set_sampler_play_mode(deck_id, mode)`
- `trigger_sampler(deck_id, slot)` / `end_sampler(deck_id, slot)`
- `set_sampler_slot_auto_gain(deck_id, slot, db)`
- `set_sampler_strip_route(route)` — `BeforeStrip` | `AfterStrip` (default `BeforeStrip`)

`DspEngine` / `Mixer` expose `lane(deck).sampler(_mut)` instead of `mixer.sampler()`.

### App / settings

- Add `sampler_strip_route: "before" | "after"` (serde snake_case), default `"before"`.
- Settings UI: Sampler section select “Through channel strip” / “After channel strip” (or Before/After wording).
- `load_bank_into_engine` loads into the **deck** that needs the bank (caller’s deck), not a single global cache.
- Trigger already passes `deck_id`; wire that through to engine.

### Memory

Two lanes × 8 slots of `Arc<LoadedAudio>` — sharing `Arc` across decks when the same file is assigned is fine; each lane still has its own voice state so simultaneous playback works.

## Non-goals

- Per-slot strip routing.
- Separate sampler bus / aux send (can come later).
- Changing bank persistence / library schema.

## Files likely touched

- `crates/engine-dsp/src/mixer_lane.rs`, `mixer.rs`, `lib.rs`, `sampler.rs` (route enum if colocated)
- `crates/engine-core/src/engine.rs`
- `apps/gui-app/src-tauri/src/deck_sampler.rs`, `lib.rs` (settings)
- `apps/gui-app/src/types.ts`, `busSettings.ts`, `SettingsAudioPanel.tsx`
- `.cursor/rules/engine-dsp.mdc` (mention lane-owned sampler)

## Acceptance

- [x] No sampler on `Mixer`; each lane has one.
- [x] Default route = before strip; setting flips behavior.
- [x] Deck A and Deck B can trigger the same sample concurrently.
- [x] Assign/trigger/clear/play-mode are per `deck_id`.
- [x] Existing sampler bank persist / UI still works.
