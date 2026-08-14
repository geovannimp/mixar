# Flutter library UI wiring (collections + drive + artwork)

Date: 2026-08-13  
Status: accepted (user waived further review gates until PR)  
Depends: `2026-08-13-flutter-library-transport-parity-design.md`, `2026-08-10-flutter-library-browse-design.md`

## Goal

Wire Flutter shell library UI to the existing FRB `LibraryTransport` surface (add folder, events, analyze/refresh, resolve paths, `getTrack` artwork) and add drive browse via a **shared `fs-browser` crate** used by Tauri and Flutter. No load-to-deck this pass.

## Decisions

| Topic | Choice |
|--------|--------|
| State | Grow Riverpod providers (no Zustand port) |
| Drive FS | Extract `crates/fs-browser` from Tauri `fs_browser.rs`; thin wrappers in both hosts |
| Load-to-deck | Out of scope |
| Artwork | Column in track table; `getTrack` only for **visible** row ids; cache by track id |
| Analyze / refresh | Row actions menu (parity with Tauri analyze; refresh included) |
| Add folder | Desktop folder picker → `addFolderCollection` → invalidate collections/tracks |
| Events | Long-lived `subscribeEvents` → invalidate / patch track rows on `trackUpdated` / `trackAnalyzed`; surface `error` / `notice` |

## Architecture

```text
crates/fs-browser
  list_volumes() / browse_directory(path)
  VolumeInfo, FsEntry, DirectoryListing
  depends: library-core (is_supported_audio_path)

Tauri gui-app
  commands call fs_browser::* (delete local copy)

host-flutter FRB
  list_fs_volumes / browse_fs_directory  (or FsBrowser::* methods)
  existing LibraryTransport unchanged for library RPCs

Flutter (Riverpod)
  libraryTransportProvider (existing)
  libraryEventsProvider — subscribe once; invalidate collections/tracks; merge track patches
  artworkCacheProvider — Map<trackId, Uint8List?>; fetch missing for visible ids
  driveVolumes / driveListing providers — FRB fs APIs
  resolveTracksForPaths when drive audio listing changes
```

### Artwork (visible rows)

1. Track table reports visible row track ids (trina scroll/viewport or on-demand when rows mount).
2. Provider diffs against cache; for each missing id calls `getTrack` (artwork only needed; full summary ok).
3. Cap concurrency (small pool, e.g. 2–4) and cancel/ignore results for ids that scrolled away before completion.
4. Lists stay artwork-free from `listCollectionTracks`; never N-file-tag on list RPC.

### Drive tab

- Source tabs: Collections | Drive (Forui tabs or equivalent).
- Drive: volume selector + directory list + audio files table (reuse track table shape where cheap; drive rows may show resolved library metadata when `resolveTracksForPaths` hits).
- No load-to-deck / drag from drive this pass.
- Folder collection “browse path” can open Drive at that path (optional nicety if cheap).

### Error handling

- Transport / picker / FS errors → destructive text or Forui toast if already used in app; keep inline error in pane if no toast pattern yet.
- Bus `Error` / `Notice` → same channel; analyzing id cleared on analyze terminal evt.

## Non-goals

- Engine load-to-deck / `EngineTransport` expansion
- Persisting artwork in `library.db`
- Shared crate extract of library transport itself
- Drive drag-to-deck / sampler assign
- Web/library on web (remains desktop-only)

## Testing

- Rust: move/adapt any Tauri fs_browser unit tests into `fs-browser`; host-flutter smoke for list/browse if easy with tempfile.
- Dart: provider unit tests where pure (artwork cache visibility diff); manual desktop smoke for picker + drive + analyze.

## Success criteria

- Add folder creates collection and shows tracks
- Analyze/refresh update BPM/key (etc.) via events without full app restart
- Artwork thumbnails appear for scrolled-into-view rows only
- Drive lists volumes/dirs/audio with same audio filter as Tauri
- Tauri still lists volumes/dirs after fs-browser extract
