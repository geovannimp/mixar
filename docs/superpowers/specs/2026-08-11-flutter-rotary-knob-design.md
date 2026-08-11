# Flutter rotary knob (Tauri parity)

Date: 2026-08-11  
Status: accepted  
Depends: Flutter desktop host (#143)

## Goal

Port the Tauri `RotaryKnob` to Flutter as a reusable widget and wire it into `MixerStrip` Gain/Hi/Mid/Low placeholders with local state only (no engine).

## Decisions

| Topic | Choice |
|--------|--------|
| Scope | Widget + MixerStrip local state |
| Look | Tauri geometry/interaction; Forui theme colors |
| Interaction | Vertical drag + step snap only (no keyboard in this slice) |
| Implementation | `CustomPaint` + pointer gestures |
| Value domain | `0..1` with center detent `0.5`, step `0.1/48` (matches Tauri `CONTROL_NORM_*`) |
| Engine / FRB | Out of scope |

## Behavior (match Tauri)

- Travel arc: −135° … +135° (270°).
- Value fill from `center` when set (bipolar); otherwise from min.
- Vertical drag: `ΔY / 72 * (max − min)`, clamped and snapped to `step`.
- Sizes: `md` (~36px dial), `sm` (~24px).
- Label above dial; raised face with tick rotated to value angle.
- `disabled` reduces opacity and ignores pointers.

## Architecture

```
apps/gui-flutter/lib/mixer/rotary_knob.dart   # RotaryKnob + paint/math
apps/gui-flutter/lib/mixer/mixer_strip.dart   # Stateful channel knobs
apps/gui-flutter/test/rotary_knob_test.dart   # paint/math + drag smoke
```

`RotaryKnob` is controlled (`value` + `onValueChange`). `MixerStrip` holds per-channel maps for Gain/Hi/Mid/Low, default `0.5`.

Colors from `FTheme` / `context.theme`: muted label, muted track, primary (or foreground) value arc, secondary face, foreground tick.

## Non-goals

- Keyboard / Semantics slider parity
- Deck accent ring colors
- Engine cmds, FLT knob, gain trim elsewhere
- New dependencies

## Acceptance

- [ ] Dragging a MixerStrip knob updates the tick/arc locally
- [ ] Center detent fill grows from mid when value ≠ 0.5
- [ ] Widget unit test covers angle/fill math and a drag updates value
- [ ] Existing mixer shell widget test still passes
