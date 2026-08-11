# Flutter library browse (collections + track table)

Date: 2026-08-10  
Status: accepted  
Depends: Flutter desktop host (#143), shared Tauri `library.db`

## Goal

Wire the Flutter mixer library panel to the real library for **browse-only**: flat collections list + track table with client-side filter. No drive/FS, analyze, artwork, or load-to-deck.

## Decisions

| Topic | Choice |
|--------|--------|
| Scope | Browse only (collections + tracks) |
| DB | Shared with Tauri via same app support dir |
| Path | Dart [`path_provider`](https://pub.dev/packages/path_provider) `getApplicationSupportDirectory()` → `library.db`; pass path into Rust |
| App ID | Align Flutter desktop IDs to Tauri `com.geovanni.gui-app` so support dirs match |
| Host API | FRB struct [`LibraryTransport`](https://cjycode.com/flutter_rust_bridge/guides/functions/methods) with methods (generated Dart class) |
| State | [Riverpod](https://riverpod.dev/) |
| Left UI | [FItemGroup](https://forui.dev/docs/widgets/data/item-group) flat collections |
| Right UI | [trina_grid](https://github.com/doonfrs/trina_grid) + filter field |
| MessagePack / streams | Reserved for later `library://bus` (`StreamSink`); **not** used for list RPCs in this slice |

## Architecture

```
main: RustLib.init → path_provider support dir → LibraryTransport.open(dbPath)
Riverpod: transport / collections / selectedId / tracks / filter
LibraryPanel: FItemGroup | filter + TrinaGrid
```

**Rust (`crates/host-flutter`)**  
- `LibraryTransport` (opaque) owns `LibraryManager` (Mutex).  
- `open(db_path: String) -> Result<Self, String>`  
- `open_in_memory() -> Result<Self, String>` (tests)  
- `list_collections() -> Result<Vec<LibraryCollectionSummary>, String>`  
- `list_collection_tracks(collection_id) -> Result<Vec<LibraryTrackSummary>, String>`  
- DTOs mirror Tauri summaries (id, name, kind, path, track_count; track title/artist/bpm/key/duration_ms/…).

**Dart**  
- Generated `LibraryTransport` from FRB.  
- Providers wrap async calls; filter is local.  
- Panel shows loading/error from `AsyncValue`; empty states when none selected.

## Non-goals

- Drive / FS volumes, add collection, analyze, bus subscribe, load-to-deck  
- Concurrent Tauri + Flutter writers on the same DB (document prefer one host)

## Acceptance

- [ ] With existing Tauri `library.db`, Flutter lists the same collections and tracks  
- [ ] Filter narrows grid rows client-side  
- [ ] `cargo test -p host_flutter` covers open_in_memory list APIs  
- [ ] Widget test with stubbed/overridden providers covers selection → grid  
- [ ] Flutter app IDs set to `com.geovanni.gui-app` on Linux/macOS (Windows VERSIONINFO aligned as needed)
