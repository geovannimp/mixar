# Track drag-and-drop via @dnd-kit/react

Date: 2026-07-25  
Status: implemented  
Scope: Library track → deck / sampler; OS files via Tauri hit-test bridge

## Goal

Replace HTML5 `DataTransfer` track dragging with `@dnd-kit/react` for in-app library → deck/pad. OS desktop file drops use Tauri `tauri://drag-*` + registered `TrackDropZone` hit-testing, then the same `applyTrackDrop`.

Out of scope: rotary knobs, Tauri window drag region.

## Design

### Provider placement

Wrap decks + library in `TrackDragProvider` (`DragDropProvider`). Waveforms stay outside so drag state does not re-render them.

### Shared `TrackDropZone`

- **dnd-kit** `useDroppable` — library track drops (`isDropTarget`).
- **Tauri registry** — `registerOsFileDropTarget` for OS hover/drop hit-test (`osHover`).
- Higher `collisionPriority` on sampler pads so nested pads win.
- `pointerIntersection` so the mixer between decks does not false-hit Deck B.

### OS files

dnd-kit cannot own desktop→app sources ([#338](https://github.com/clauderic/dnd-kit/issues/338)). A ghost `useDraggable` + programmatic `actions.start` was tried; overlay/collisions did not activate reliably (shape gate + initializing/`move` timing). **`OsFileDropBridge`** listens to `tauri://drag-*` (`dragDropEnabled: true`), hit-tests zones, highlights, and calls `applyTrackDrop`.

### Drop handling

- In-app: `onDragEnd` → `applyTrackDrop`
- OS: bridge → `applyTrackDrop`

### Dependencies

`@dnd-kit/react`, `@dnd-kit/collision` in `apps/gui-app`.
