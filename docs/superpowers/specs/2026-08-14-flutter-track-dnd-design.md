# Flutter track drag-and-drop (library + drive + OS files)

Date: 2026-08-14  
Status: accepted (user waived remaining review gates until PR)  
Depends: `2026-08-13-flutter-engine-transport-design.md`, `2026-08-13-flutter-library-ui-wiring-design.md`  
Reference: Tauri `2026-07-25-track-dnd-kit-design.md`

## Goal

Load tracks onto Flutter decks by dragging library or drive table rows, and by dropping OS audio files onto a deck. Auto-start `EngineTransport` when the mixer is shown. Show the loaded title from engine `Updated` events.

## Decisions

| Topic | Choice |
|--------|--------|
| DnD package | `super_drag_and_drop` (in-app `localData` + inbound `Formats.fileUri`) |
| Sources | Collection rows, drive rows, OS files |
| Targets | Deck A / Deck B only |
| Sampler / file picker / outbound export | Out of scope |
| Load APIs | Existing `EngineTransport.loadLibraryTrack` / `loadPath` |
| Load rule | `source == library` and non-empty `trackId` → library load; else path load |
| Drive resolved rows | `id != path` counts as library (same as Tauri `rowToDragPayload`) |
| OS multi-file | First path with a supported audio extension; else warning toast |
| Engine start | Once, `keepAlive`, backend `"auto"`; desktop only |
| Start failure | Fatal exit on desktop (AGENTS.md) |
| Deck chrome | Title from `EngineEvt.updated.track`; `hasTrack` follows a non-empty title |
| Header | `Engine idle` / `Engine running` from status events |
| Waveforms / mixer strip | Not drop targets |

## Architecture

```text
engineTransportProvider     LibraryTransport → EngineTransport.start (keepAlive)
engineEventsBootstrap       subscribeEvents → running + per-deck title
engineRunningProvider
deckTrackTitleProvider(deckId)

Track table rowWrapper
  DragItemWidget(localData: TrackDragPayload map)
    DraggableWidget(row)
  only when engine is running (keeps widget tests plugin-free)

DeckPanel
  DropRegion(formats: [Formats.fileUri])
    local payload → applyTrackDrop
    fileUri → first supported audio path → loadPath
  only when engine transport is present
```

`applyTrackDrop` is a pure routing helper plus async load calls. OS paths use the same `library-core` extension list as Tauri’s fallback (`mp3`, `flac`, `wav`, `aiff`, `aif`, `ogg`, `m4a`, `aac`, `opus`, `wma`, `alac`).

In-app drags do not add file formats (no export to the desktop). Drop regions list `Formats.fileUri` so OS files are accepted; `localData` is still visible on in-app drops.

## Behavior

- Drag disabled until the engine is running.
- Drop highlight: inset primary/emerald border while a compatible drag is over the deck.
- Unsupported OS drop (no audio files): `showFToast` warning — “No supported audio files in drop”.
- Load / runtime engine errors: destructive toast (or existing inline message). Do not crash after a successful start.
- Widget tests override `engineTransportProvider` to skip start so `AppShell` stays plugin-free.

## Non-goals

- Sampler pad assign
- File-picker load button
- Play/pause/cue wiring
- OS-file export from the library
- Web load-to-deck

## Testing

- Dart unit: payload parse, library-vs-path routing, audio extension filter, OS path → payload, engine snapshot reducer.
- Widget: existing mixer shell test overrides engine transport; deck title override shows loaded name.
- Manual desktop: library row → deck, drive row → deck, `.wav` from file manager → deck.

## Success criteria

- [ ] Mixer auto-starts the engine; header shows running
- [ ] Library and drive rows drag onto Deck A/B and load
- [ ] OS audio file drop onto a deck loads via `loadPath`
- [ ] Deck subtitle shows the loaded title
- [ ] `flutter test` in `apps/gui-flutter` passes
