# Widen loop / beat-jump beats to f32

Date: 2026-08-07  
Issue: [#137](https://github.com/geovannimp/rust-dj-engine/issues/137)  
Related: `docs/superpowers/specs/2026-08-06-controller-action-args-design.md`  
Status: approved (approach: plain `f32`)

## Goal

Allow fractional beat sizes on the engine wire and in controller map args (decimal floats), so maps can express e.g. `auto_loop(beats:0.25)` and `beat_jump(beats:-0.5)`.

## Scope

**In:**

- `CmdBody::SetAutoLoop { beats: f32 }`
- `CmdBody::BeatJump { beats: f32 }` (sign preserved; negative = back)
- `CmdBody::BeginLoopRoll { beats: f32 }`
- Matching `Engine` methods in `engine-core`
- FE zod schemas in `apps/gui-app/src/lib/engine/wire.ts`
- Controller arg parse/resolve: decimal floats + integers coerce to `f32`
- Tests: `bus_performance`, `bus_pads`, controller parse/catalog/resolve, GUI wire if typed

**Out:**

- Rational `1/4` grammar in action ids (still deferred)
- Hot-cue `loop_length_beats` (stays integer metadata)
- Changing UI pad preset lists (may remain whole beats)

## Validation

Trust boundary (controller catalog at load + engine methods on apply):

| Command | Rule |
|---------|------|
| `SetAutoLoop` / `BeginLoopRoll` / map `auto_loop` | finite and `beats > 0` |
| `BeatJump` / map `beat_jump` | finite and `beats != 0` |

Reject NaN / ±Inf. No artificial upper cap (engine may still clamp loop out to track duration as today).

## Architecture

Plain `f32` on the wire (no newtype). Msgpack encodes a float; existing integer literals in tests remain valid if encoders accept them, but Rust types and FE schemas use float.

Controller:

- Extend `ArgValue` with `Float(f32)`; drop `Eq` on `ArgValue` / `ActionArgs` (keep `PartialEq`).
- `parse_arg_value`: try `i64` first, then finite `f32` decimal (`0.25`, `-0.5`), then ident.
- `require_f32(key)`: accept `Int` or `Float`; error on ident / missing.
- Catalog + `resolve_action` use `require_f32` for `beats` with the rules above.

Engine duration math already uses `f64::from(beats) * (60/bpm)`; swap `u32`/`i32` params for `f32` and keep the same formulas (`beat_jump` uses signed `f32` as `f64`).

FE: replace `.int()` on those three cmd beats fields with finite number checks matching the rules (positive / non-zero).

## Non-goals

- New beat-size UI chrome
- Changing pad bank hardcoded beat tables unless compile errors force a cast
