# Mixer Layout Grid Implementation Plan

> **For agentic workers:** Implement task-by-task. User waived plan review — proceed to code.

**Goal:** Fixed 410px deck/mixer row with true mixer columns (HI↔EQ stack, GAIN↔fader) and no mixer clipping.

**Architecture:** Drop inner waveform↔decks resize; decks row is `h-[410px] shrink-0`. Rebuild `DeckMixer` as per-channel EQ + fader columns with meters between faders; height from content.

**Tech Stack:** React, Tailwind, existing `RotaryKnob` / `Slider` / `LevelMeter`.

## Global Constraints

- Deck row height fixed at **410px** (not resizable).
- HI aligned with MID/LOW/FLT; GAIN aligned with volume fader.
- Mixer height follows content; no `overflow-hidden` clip on mixer chrome.
- GUI only; no engine changes.

---

### Task 1: Fixed 410px decks row in `DeckGrid`

- Replace waveform/decks `ResizablePanelGroup` with flex column: waveform `flex-1 min-h-[70px]`, decks `h-[410px] shrink-0`.
- Bump `MixerPage` decks panel `minSize` so waveform + 410px fit (e.g. ≥ 480px).

### Task 2: Column mixer in `DeckMixer`

- Fold HI into EQ column; put GAIN atop fader column; remove `MixerTopKnobRow`.
- Meters between faders with GAIN/%/cue spacers; crossfader below; content-sized root (`overflow` not clipping).

### Task 3: Verify

- Lint touched files; visual check: 410px row, aligned columns, full crossfader.
