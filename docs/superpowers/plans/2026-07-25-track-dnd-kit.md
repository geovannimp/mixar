# Track DnD (@dnd-kit + react-dropzone) Implementation Plan

> **For agentic workers:** implement task-by-task; mark checkboxes done as you go.

**Goal:** Shared `TrackDropZone` + `TrackDragProvider`; migrate library→deck/sampler off HTML5 MIME drag.

**Architecture:** MixerPage wraps content in provider. Rows are `useDraggable`. Decks/pads use `TrackDropZone` (dnd-kit droppable + react-dropzone).

## File map

| File | Role |
|------|------|
| `src/lib/trackDrag.ts` | Ids, typed data, OS path helper |
| `src/components/TrackDropZone.tsx` | Shared drop wrapper |
| `src/components/TrackDragProvider.tsx` | Provider + overlay + onDragEnd |
| `src/pages/MixerPage.tsx` | Wrap with provider |
| `LibraryTrackTable.tsx` | useDraggable rows |
| `DeckPanel.tsx` | TrackDropZone |
| `SamplerPads.tsx` | TrackDropZone per pad (+ deckId) |
| `libraryTable.ts` / `trackDragPreview.tsx` | Remove dead HTML5 helpers |

## Tasks

### Task 1: Dependencies + types + TrackDropZone

- [x] Install `@dnd-kit/react` and `react-dropzone`
- [x] Add `trackDrag.ts`
- [x] Add `TrackDropZone.tsx`

### Task 2: Provider + wire surfaces

- [x] Add `TrackDragProvider.tsx`
- [x] Wrap `MixerPage`
- [x] Migrate library rows, `DeckPanel`, `SamplerPads`

### Task 3: Cleanup + verify

- [x] Delete unused HTML5 track drag helpers
- [x] Lint / typecheck affected files
