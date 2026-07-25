# Track drag-and-drop via @dnd-kit/react

Date: 2026-07-25  
Status: draft  
Scope: Library track → deck load and sampler-pad assign (in-app only)

## Goal

Replace HTML5 `DataTransfer` track dragging with `@dnd-kit/react` so library rows, deck surfaces, and sampler pads share one drag context. Fix class of bugs where nested drop targets both receive the same native drop.

Out of scope: rotary knobs, Tauri window drag region, OS file drops from the desktop.

## Current behavior

- `LibraryTrackTable` sets `draggable` and writes a JSON payload via `writeTrackDragData` / `startTrackDrag`.
- `DeckPanel` accepts drops to load a track; uses enter/leave depth counting for highlight.
- `SamplerPads` accepts drops per slot; `stopPropagation` prevents the deck from also loading.
- Preview: `TrackDragCard` rendered into a drag image host.

## Design

### Provider placement

Wrap `MixerPage` content in a `TrackDragProvider` (`DragDropProvider` from `@dnd-kit/react`) so library, decks, and pads share one context.

### Drag payload

Keep `TrackDragPayload` as the typed payload. Attach it on the draggable via dnd-kit `data` (not MIME types).

```ts
type TrackDragData = {
  type: "track";
  payload: TrackDragPayload;
};
```

Droppable `data` identifies the target:

```ts
type DeckDropData = { type: "deck"; deckId: number };
type SamplerDropData = { type: "sampler"; deckId: number; slot: number };
```

Stable droppable ids, e.g. `deck:${deckId}`, `sampler:${deckId}:${slot}`.

### Draggables

Library table rows use `useDraggable` when the engine is running and the row is not analyzing. Disable otherwise (same rules as today).

### Droppables

- Entire `DeckPanel` section: `useDroppable` for deck load; `isOver` drives the existing emerald inset highlight.
- Each sampler pad: `useDroppable`; `isOver` drives the pad highlight.
- Nested collision: prefer the deepest / most specific droppable (sampler pad over deck). Configure collision detection accordingly (pointer within bounds / prefer nested target). When a pad is the target, only assign the sample — do not also load the deck.

### Drop handling

Central `onDragEnd` in `TrackDragProvider`:

1. If canceled or no target → no-op.
2. If target is sampler → `assignSamplerFromTrack` / `assignSamplerFromPath`.
3. If target is deck → `loadLibraryTrackToDeck` / `loadPathToDeck`.
4. Respect existing enablement (engine running, deck not busy, pad not disabled).

Callbacks can be passed into the provider or invoked via existing `engineActions` so panels stay thin.

### Overlay

Use dnd-kit `DragOverlay` (or equivalent) rendering `TrackDragCard` from the active drag data. Remove HTML5 `setDragImage` / `trackDragPreview.tsx` once unused.

### Cleanup

Remove unused helpers after migration:

- `writeTrackDragData`, `readTrackDragData`, `acceptsTrackDrag` (if nothing else uses them)
- `startTrackDrag` / `trackDragPreview.tsx`
- Native `onDrag*` handlers on deck / sampler / library rows

Keep `parseTrackDragPayload` / `rowToDragPayload` / `TrackDragPayload` as pure data helpers.

### Dependency

Add `@dnd-kit/react` to `apps/gui-app`.

## Success criteria

- Drag library track onto deck → loads that deck only.
- Drag onto sampler pad → assigns that pad only (deck does not load).
- Visual highlight on active drop target (deck or pad).
- Drag overlay shows track card.
- No HTML5 track drag path left for this flow.

## Non-goals

- Sortable library rows
- Cross-window / OS file DnD
- MIDI or keyboard pad-mode cycling changes
