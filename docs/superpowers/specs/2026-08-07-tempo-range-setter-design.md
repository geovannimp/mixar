# Tempo range setter (percent)

Date: 2026-08-07  
Issue: [#140](https://github.com/geovannimp/rust-dj-engine/issues/140)  
Status: approved (implement)  
Depends on: control value normalization (#138 / #139)

## Goal

Make per-deck `tempo_range` configurable from the GUI and controllers. Store and wire it as a **pitch fraction** (industry-standard percent of track rate), not a BPM half-span. Show pitch **percent** offset in the tempo panel. Default ±6%; cycle ±6% / ±10% / ±16% / ±25% (Mixxx/DDJ-400).

## Decisions

| Topic | Choice |
|-------|--------|
| Unit | Fraction on wire/DSP (`0.06` = ±6%) |
| Steps | `0.06`, `0.10`, `0.16`, `0.25`; default `0.06` |
| On range change | Keep fader `speed` `0..1`; UI knob mirrors engine `speed` |
| Sync + range | Do **not** clear `ratio_override`; remap displayed `speed` from override through new range |
| Sync + sliders | Master `playback_ratio` drives slaves; each deck remaps `speed` via **its** `tempo_range` so both knobs follow |
| Soft-takeover | Unchanged latch on `speed`; HW catches when positions diverge |
| GUI | Cycle button labeled `±6%` (etc.) in tempo panel |
| Controller | DDJ-400: Shift+BEAT SYNC = ch N note `0x60` → cycle next step |
| LED | Out of scope (no HW range LED on DDJ-400) |

## Wire

- `Kind::SetTempoRange` / `CmdBody::SetTempoRange { tempo_range: f32 }` (no soft_takeover).
- `DeckSnapshot` / `DeckUpdated` / status include `tempo_range: f32`.
- Reject non-finite / `<= 0` values.

## DSP

```text
playback_ratio = 1 + (0.5 - speed) * 2 * tempo_range   // clamp ratio > 0
speed = clamp(0.5 - (ratio - 1) / (2 * tempo_range), 0, 1)
```

Effective BPM for display/sync still uses track BPM × ratio. Track BPM is **not** used to scale the fader span.

`set_tempo_range(r)`:

1. `tempo_range = r.max(eps)` (or reject ≤0 at cmd layer).
2. If `ratio_override` is set, recompute `speed` from that ratio + new range.
3. Else leave `speed` unchanged (playback ratio changes with the new span).

## Controller

- Alias `tempo_range` (or `cycle_tempo_range`) on deck.
- Action `Deck(_)::cycle_tempo_range` → read snapshot range, advance in step list, publish `SetTempoRange`.
- DDJ-400 `device.toml`: `tempo_range = { type = "note", channel = 1|2, note = 0x60 }` (distinct from pad bank ch8 `0x60`).
- `map.toml`: bind to `Deck(_)::cycle_tempo_range`.

## GUI

- Tempo panel: replace BPM pitch offset with `formatPitchPercent(speed, tempo_range)`; add cycle control calling `set_tempo_range` with next step.
- Mirror `tempo_range` on deck status from bus events.
- Shared TS constants for default + steps matching Rust.

## Out of scope

Range LEDs; persisting range per track/session; arbitrary ranges outside the four steps in the GUI (engine may still accept any `> 0`).
