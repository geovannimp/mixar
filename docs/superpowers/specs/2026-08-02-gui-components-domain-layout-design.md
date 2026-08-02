# gui-app components domain layout

Date: 2026-08-02  
Status: implemented  
Scope: Reorganize `apps/gui-app/src/components` into feature-domain folders (same idea as `hooks/engine` and `hooks/library`)

## Goal

Make component ownership obvious by domain, reduce the flat `components/` root (~40 files), and keep imports concrete (no barrels). Behavior and visuals stay unchanged — move files + update import paths only.

## Non-goals

- No UI redesign, prop API changes, or state/store refactors.
- No barrel `index.ts` / directory imports (`@/components/deck` stays forbidden).
- No oxlint `no-restricted-imports` / `no-barrel-file` rules for this (convention lives in `.cursor/rules/gui-app.mdc`).
- Do not merge `ui/` into domains; design-system primitives stay under `ui/`.
- Do not move `settings/` (already domain-scoped).

## Approach

**Feature-domain folders**, with pads nested under deck and modals under `dialogs/`.

| Folder | Responsibility |
|--------|----------------|
| `ui/` | Shared primitives (coss-style). Unchanged. |
| `settings/` | Settings page panels/fields. Unchanged. |
| `deck/` | Per-deck chrome and performance panels. |
| `deck/pads/` | Pad-mode grids (today’s `deck-pads/`). |
| `mixer/` | Channel strip, meters, knobs, cue-mix controls. |
| `library/` | Library pane, collections, drive browser, track table. |
| `waveform/` | Dual-deck waveform, lanes, markers. |
| `shell/` | App chrome (header, window controls, status). |
| `dnd/` | In-app track drag + OS file drop bridge. |
| `dialogs/` | Modal dialogs (not popovers/menus). |

## Target membership

### `deck/`

- `DeckGrid.tsx`
- `DeckPanel.tsx`
- `DeckTransport.tsx`
- `DeckTempoPanel.tsx`
- `DeckLoopPanel.tsx`
- `DeckPadsPanel.tsx`
- `DeckTrackInfo.tsx`
- `DeckInfoPopover.tsx`
- `DeckOverviewPreview.tsx`

### `deck/pads/` (from `deck-pads/`)

- `PadGridContainer.tsx`
- `HotCuePads.tsx`
- `LoopRollPads.tsx`
- `BeatJumpPads.tsx`
- `SamplerPads.tsx`

### `mixer/`

- `DeckMixer.tsx`
- `LevelMeter.tsx`
- `RotaryKnob.tsx`
- `HeadphoneMonitorControls.tsx`

### `library/`

- `LibraryPanel.tsx`
- `LibraryPane.tsx`
- `LibraryTrackTable.tsx`
- `LibrarySourceTabs.tsx`
- `CollectionList.tsx`
- `DriveBrowser.tsx`
- `DriveFolderRow.tsx`
- `DrivePathBreadcrumbs.tsx`
- `DriveSelector.tsx`
- `TrackActionsMenu.tsx`
- `MessageBanner.tsx`

### `waveform/`

- `DualDeckWaveform.tsx`
- `RustRenderedLane.tsx`
- `useLaneDimensions.ts`
- `WaveformCueMarkers.tsx`
- `WaveformWindowMarkers.tsx` (if still present / unused cleanup optional)
- `WaveformWindowMarkersMotion.tsx`
- `WaveformWindowMarkersMotion.test.ts` (colocate with module)

### `shell/`

- `AppHeader.tsx`
- `TitleBarDragRegion.tsx`
- `WindowTitleBarControls.tsx`
- `WindowResizeBorder.tsx`
- `StatusPill.tsx`

### `dnd/`

- `TrackDragProvider.tsx`
- `TrackDragCard.tsx`
- `TrackDropZone.tsx`
- `OsFileDropBridge.tsx`

### `dialogs/`

- `SamplerBankConfigDialog.tsx`
- Future modals only (not popovers like `TrackActionsMenu` / `DeckInfoPopover`)

## Import rules

- Always import the concrete file: `@/components/deck/pads/HotCuePads`, never `@/components/deck` or `@/components/deck/pads`.
- Prefer `@/` aliases for cross-folder imports; relative `./` is fine within the same folder.
- After the move, update `.cursor/rules/gui-app.mdc`:
  - Document domain folders.
  - Change pad path example from `deck-pads/` → `deck/pads/`.

## Dependency direction (keep acyclic)

Allowed edges (high level):

- `pages` / `layouts` → any domain
- `deck` → `deck/pads`, `dnd` (`TrackDropZone`), `dialogs` (sampler bank config), `ui`
- `mixer` → `ui` (and may be imported by `shell` for cue mix)
- `library` → `ui`, `dnd` only if needed later
- `waveform` → `ui` if needed; no dependency on `library` UI (hooks already supply track data)
- `dnd` → `ui` (toast) only as today
- `dialogs` → `ui`; may be imported by `deck/pads`
- Domains must not import `pages` / `layouts`

Popovers stay with their feature (`library/TrackActionsMenu`, `deck/DeckInfoPopover`).

## Migration plan

1. `git mv` files into the folders above (preserve history).
2. Update all imports (`rg` for old paths: `@/components/Deck`, `@/components/deck-pads`, relative `./Deck`, etc.).
3. Update `gui-app.mdc` pad/domain notes.
4. `npx tsc --noEmit` + `npx oxlint` + `npm test` in `apps/gui-app`.
5. Smoke: mixer page loads; pads open sampler bank dialog; library drag-drop still works.

Optional follow-up (out of this move): delete dead `WaveformWindowMarkers.tsx` if unused after waveform folder settle.

## Success criteria

- `components/` root has no feature `.tsx` left (only domain folders + maybe nothing else).
- No new barrel files.
- App typechecks and existing gui-app tests pass.
- Import paths are greppable by domain (`@/components/mixer/…`, `@/components/library/…`).
