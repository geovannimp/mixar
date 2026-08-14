# Flutter Track Drag-and-Drop Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans (this session: inline; user waived review checkpoints until PR).

**Goal:** Library + drive row drag and OS audio file drop onto Flutter decks, with auto-started engine and loaded title.

**Architecture:** `super_drag_and_drop` `DragItemWidget` / `DropRegion`; Riverpod `EngineTransport` keepAlive; shared `applyTrackDrop` routing matching Tauri.

**Tech Stack:** Flutter, Riverpod 3, Forui, trina_grid `rowWrapper`, `super_drag_and_drop` 0.9.x, existing FRB `EngineTransport`.

## Global Constraints

- No sampler assign, file picker, play/pause wiring, or OS export.
- Desktop only; widget tests override `engineTransportProvider` to `null`.
- Audio extensions match `library-core` `SUPPORTED_AUDIO_EXTENSIONS`.
- Cargo via `crates/Cargo.toml`; Flutter via `mise exec -- flutter …` in `apps/gui-flutter`.
- Shortest working diffs.

## File map

| Path | Role |
|------|------|
| `apps/gui-flutter/lib/mixer/track_drag.dart` | Payload, parse, filter, apply routing |
| `apps/gui-flutter/lib/mixer/engine_providers.dart` | Start, events, running, titles |
| `apps/gui-flutter/lib/mixer/track_drop_zone.dart` | Deck `DropRegion` + highlight |
| `apps/gui-flutter/lib/shell/desktop*.dart` | `fatalExit` for start failure |
| `apps/gui-flutter/lib/library/track_table_pane.dart` | `rowWrapper` drag source |
| `apps/gui-flutter/lib/mixer/deck_panel.dart` / `deck_grid.dart` | `deckId`, title, drop zone |
| `apps/gui-flutter/lib/shell/app_header.dart` / `app_shell.dart` | Status + bootstrap |
| `apps/gui-flutter/pubspec.yaml` | `super_drag_and_drop` |
| `apps/gui-flutter/test/track_drag_test.dart` | Pure routing / filter / snapshot |

---

### Task 1: Pure track-drag helpers (TDD)

**Files:**
- Create: `apps/gui-flutter/lib/mixer/track_drag.dart`
- Test: `apps/gui-flutter/test/track_drag_test.dart`

**Interfaces:**
- Produces: `TrackDragPayload`, `payloadFromLibraryTrack`, `payloadFromOsPath`, `parseTrackDragLocalData`, `filterAudioFilePaths`, `applyEngineEvt`, `EngineUiSnapshot`

- [ ] **Step 1:** Write failing tests for filter, OS payload, library-vs-path routing, snapshot reducer.
- [ ] **Step 2:** Implement helpers until tests pass.
- [ ] **Step 3:** `mise exec -- flutter test test/track_drag_test.dart`

### Task 2: Engine providers + fatal start

**Files:**
- Create: `apps/gui-flutter/lib/mixer/engine_providers.dart`
- Modify: `apps/gui-flutter/lib/shell/desktop.dart`, `desktop_io.dart`, `desktop_stub.dart`
- Modify: `apps/gui-flutter/lib/shell/app_shell.dart`, `app_header.dart`
- Modify: `apps/gui-flutter/test/widget_test.dart` (override transport to null)

**Interfaces:**
- Produces: `engineTransportProvider` (`FutureProvider<EngineTransport?>`), `engineRunningProvider`, `deckTrackTitleProvider(int)`, `engineEventsBootstrapProvider`

- [ ] **Step 1:** Start from library transport with `EngineStartConfig(backend: 'auto')`; `keepAlive`.
- [ ] **Step 2:** Subscribe events; patch running + titles via `applyEngineEvt`.
- [ ] **Step 3:** Desktop start failure → `fatalExit(1)`. Tests override provider.
- [ ] **Step 4:** Header text from `engineRunningProvider`. Watch bootstrap from `AppShell`.

### Task 3: Drop zone + row drag + wire decks

**Files:**
- Create: `apps/gui-flutter/lib/mixer/track_drop_zone.dart`
- Modify: `apps/gui-flutter/pubspec.yaml`
- Modify: `apps/gui-flutter/lib/library/track_table_pane.dart`
- Modify: `apps/gui-flutter/lib/mixer/deck_panel.dart`, `deck_grid.dart`

**Interfaces:**
- Consumes: Task 1 payload + Task 2 engine providers / `loadLibraryTrack` / `loadPath`
- Produces: `TrackDropZone(deckId:)` wrapping deck chrome

- [ ] **Step 1:** `flutter pub add super_drag_and_drop`
- [ ] **Step 2:** `TrackDropZone` — `DropRegion`, highlight, local + fileUri, toast if no audio.
- [ ] **Step 3:** Trina `rowWrapper` `DragItemWidget` when engine running; store `path` on the row.
- [ ] **Step 4:** Deck A = 0, Deck B = 1; show title; `hasTrack` from title.

### Task 4: Verify

- [ ] **Step 1:** `mise exec -- flutter test`
- [ ] **Step 2:** `mise exec -- flutter analyze` on touched Dart if cheap
- [ ] **Step 3:** PR
