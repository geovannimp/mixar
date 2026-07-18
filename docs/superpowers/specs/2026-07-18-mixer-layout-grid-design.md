# Mixer Layout Grid Design

**Date:** 2026-07-18  
**Scope:** GUI only (`apps/gui-app`) — visual layout of the deck/mixer row

## Goal

Fix the deck/mixer/deck strip to a fixed **410px** height, realign mixer knobs into true columns (HI with MID/LOW/FLT; GAIN with the volume fader), and stop the mixer from clipping after level meters were added.

## Requirements

| Decision | Choice |
|----------|--------|
| Deck row height | Fixed **410px** (not resizable; panel `disabled` with min=max=410) |
| Waveform row | Vertically resizable via handle above the decks row (trades space with library) |
| Mixer grid | CSS Grid / column-based layout (Approach 1) |
| HI alignment | Same column as MID, LOW, FLT |
| GAIN alignment | Same column as volume fader (% + cue) |
| Level meters | Stay between the two volume fader columns |
| Mixer sizing | Height follows content; no fixed inner height that clips |
| Deck panels | Fill the fixed 410px row; controls unchanged |

Out of scope: engine/DSP changes, meter ballistics, EQ behavior, persisting layout sizes, redesign of deck pads/jog/tempo.

## Problem

Today (`DeckGrid` + `DeckMixer`):

1. Deck row is a resizable panel (`min` 340px / `default` 350px) with `overflow-hidden`.
2. Mixer puts HI + GAIN in a separate top row (`MixerTopKnobRow`) while MID/LOW/FLT and faders live in a lower flex row — columns drift.
3. Mixer uses `h-full` + `overflow-hidden` inside a fixed-width strip, so adding meters squeezed the crossfader and clipped it.

## Layout

### Deck row (`DeckGrid` / `MixerPage`)

```text
┌──────────────── waveforms (resizable) ────────────────┐
├─────────────────── resize handle ─────────────────────┤
├──────────────────── h = 410px fixed ──────────────────┤
│  Deck A  │           Mixer            │  Deck B       │
├─────────────────── resize handle ─────────────────────┤
└──────────────────── library ──────────────────────────┘
```

- Single vertical `ResizablePanelGroup` on `MixerPage`: waveforms → decks → library.
- Decks panel locked at **410px** (`minSize` = `maxSize` = `410px`, `disabled`).
- Dragging either handle resizes waveform ↔ library; decks stay fixed.
- `DeckGrid` is only the deck/mixer/deck row.

### Mixer (`DeckMixer`)

```text
  Mixer [M/S]
┌──────┬─────┬──┬──┬─────┬──────┐
│  HI  │GAIN │  │  │GAIN │  HI  │
│ MID  │     │mA│mB│     │ MID  │
│ LOW  │ fad │  │  │ fad │ LOW  │
│ FLT  │  %  │  │  │  %  │ FLT  │
│      │ cue │  │  │ cue │      │
├──────┴─────┴──┴──┴─────┴──────┤
│           Crossfader          │
└───────────────────────────────┘
```

- **EQ column (per deck):** HI, MID, LOW, FLT in one vertical stack (`w-12`).
- **Fader column (per deck):** GAIN, vertical volume slider, percent, cue (`w-10`).
- **Meters:** dual `LevelMeter` between fader columns; vertically aligned with the slider track (same spacer pattern as today for % / cue if needed).
- **Crossfader:** full-width row below channels.
- Root mixer: content-sized height (`h-auto` / no `overflow-hidden` clip). Width stays compact (`~12.5rem` or whatever the columns need).

### Implementation notes

- Collapse `MixerTopKnobRow` + `DeckEqColumn` into per-channel columns (or one CSS grid) so HI is not a sibling of a different flex tree.
- Keep existing knob/fader components and engine hooks; only structure/classes change.
- Prefer `overflow-hidden` only on deck panels if needed for their internal scroll; mixer should not clip its own chrome.

## Success criteria

1. Deck/mixer/deck row is always **410px** tall and not vertically resized.
2. HI sits on the same vertical axis as MID/LOW/FLT for each channel.
3. GAIN sits on the same vertical axis as that channel’s volume fader.
4. Crossfader and all knobs are fully visible (no clipping at the mixer bottom).
5. Level meters remain between the two faders; mono/stereo toggle still works.

## Files likely touched

- `apps/gui-app/src/components/DeckGrid.tsx` — fixed 410px decks row
- `apps/gui-app/src/components/DeckMixer.tsx` — column/grid restructure, content height
