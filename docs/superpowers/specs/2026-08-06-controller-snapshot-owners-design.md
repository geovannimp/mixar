# Slim ControlSnapshot: soft-takeover, toggles, idle

Date: 2026-08-06  
PR: [#132](https://github.com/geovannimp/rust-dj-engine/pull/132)  
Status: implemented (pad routing deferred)

## Goal

Move soft-takeover and toggle decisions into the engine (owners). Slim controller idle to a playing-deck count. Keep `pad_mode` / `hot_cues` mirror until pad routing is redesigned. Rhai `get_snapshot` already removed.

## Soft-takeover

- Absolute cmds gain `soft_takeover: bool` (`#[serde(default)]` → false for UI).
- When true, engine compares incoming value to current (3/127 norm threshold, same as today) and may no-op until latched; latch clears when engine value moves without a matching HW set (or on load — keep simple: per-control latch in engine).
- EQ: add `Kind::SetEqBand` + `CmdBody::SetEqBand { band, gain_db, soft_takeover }` so MIDI does not need sibling-band snapshot. Existing `SetEq` stays for UI full writes (`soft_takeover` default false).

## Toggles

| Map leaf | Engine |
|----------|--------|
| `toggle_play` | new `Kind::TogglePlay` + `Empty` |
| `set_quantize` | new `Kind::ToggleQuantize` + `Empty` (or keep SetQuantize for UI) |
| `set_headphone_cue` | new `Kind::ToggleHeadphoneCue` |
| `set_master_cue` | new `Kind::ToggleMasterCue` |
| `toggle_sync` | already `ToggleSync` |

UI can keep explicit `Set*` with `enabled`.

## Idle heartbeat

- `MappingSession` holds `playing_decks: u8` (and optional `[bool;4]` to make ++/-- idempotent per deck).
- `on_deck_playing(deck, playing)` updates count; heartbeat runs only when count == 0.
- Host should forward play/pause from engine Status/Updated (wire `ControllerEngine::on_deck_playing`); optimistic MIDI Play/Pause may also update until Status is wired.

## Out of scope

Pad routing / deleting `hot_cues`+`pad_mode` mirror; fractional beats (#137).
