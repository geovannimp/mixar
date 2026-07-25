# Deck Pads Panel — ts-pattern + Per-Mode Grids

**Date:** 2026-07-22  
**Status:** Implemented

## Goal

Refactor `DeckPadsPanel` so pad-mode UI is selected with [ts-pattern](https://github.com/gvergnaud/ts-pattern) and each pad mode owns its full 8-pad grid as a dedicated component. Behavior stays unchanged.

## Non-goals

- No changes to pad mode semantics, engine API, or sampler bank chrome beyond prop plumbing. (Deck focus / `useDeckHotkeys` were removed separately.)
- No visual redesign of pads or tabs.

## Approach

**Mode grids + single `match`:** `DeckPadsPanel` keeps mode tabs, sampler bank toolbar, and bank config dialog. The pad area is:

```tsx
{match(deck.pad_mode)
  .with("hot_cue", () => <HotCuePads ... />)
  .with("loop_roll", () => <LoopRollPads ... />)
  .with("beat_jump", () => <BeatJumpPads ... />)
  .with("sampler", () => <SamplerPads ... />)
  .exhaustive()}
```

Each mode component renders the existing `grid-cols-4` 8-pad layout and its own pad cells.

## File layout

```
apps/gui-app/src/components/
  DeckPadsPanel.tsx              # chrome + match
  deck-pads/
    PadGridContainer.tsx
    HotCuePads.tsx
    LoopRollPads.tsx
    BeatJumpPads.tsx
    SamplerPads.tsx
```

No barrel `index.ts` — import concrete modules only. Mode grids wrap pad buttons in `PadGridContainer`.

## Props

Each mode component receives only what it needs:

| Component     | Notable props |
|---------------|---------------|
| `HotCuePads`  | `hotCues`, `disabled`, trigger/save/delete callbacks |
| `LoopRollPads`| `disabled`, begin/end loop-roll callbacks |
| `BeatJumpPads`| `disabled`, `onBeatJump` |
| `SamplerPads` | `slots`, `disabled`, `holdLike`, trigger/end/clear/assign callbacks |

Shared constants (`LOOP_ROLL_BEATS`, `BEAT_JUMP_*`, accents, drag helpers) stay in existing libs; mode components import them directly.

## Dependency

Add `ts-pattern` to `apps/gui-app` dependencies.

## Acceptance

1. All four modes render and behave as today (including hold/shift/drag for sampler).
2. Adding a new `PadMode` without a `.with` arm fails typecheck via `.exhaustive()`.
3. `DeckPadsPanel` no longer contains a per-slot `switch (deck.pad_mode)`.
4. Lint/format clean for touched files.
