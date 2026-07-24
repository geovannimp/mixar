# Deck Pads ts-pattern Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Refactor `DeckPadsPanel` to select mode UI with `ts-pattern` and extract each mode’s 8-pad grid into its own component.

**Architecture:** `DeckPadsPanel` keeps tabs + sampler bank chrome; `match(deck.pad_mode).exhaustive()` renders one of four grid components under `components/deck-pads/`.

**Tech Stack:** React, ts-pattern, existing `DeckButton` / pad helpers.

---

### Task 1: Add dependency + extract mode grids + wire match

**Files:**
- Modify: `apps/gui-app/package.json` (add `ts-pattern`)
- Create: `apps/gui-app/src/components/deck-pads/PadGridContainer.tsx`
- Create: `apps/gui-app/src/components/deck-pads/HotCuePads.tsx`
- Create: `apps/gui-app/src/components/deck-pads/LoopRollPads.tsx`
- Create: `apps/gui-app/src/components/deck-pads/BeatJumpPads.tsx`
- Create: `apps/gui-app/src/components/deck-pads/SamplerPads.tsx`
- Modify: `apps/gui-app/src/components/DeckPadsPanel.tsx`
- Note: no `index.ts` barrel — import concrete modules only.

**Steps:**
1. `npm install ts-pattern -w gui-app`
2. Move each `switch` arm’s grid into the matching component (preserve behavior).
3. Replace per-slot `switch` with `match(deck.pad_mode)...exhaustive()`.
4. `npm run lint -- -w gui-app` / typecheck via `tsc` as needed.
