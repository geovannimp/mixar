import 'package:flutter/widgets.dart';
import 'package:material_ui/material_ui.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/library/history_providers.dart';
import 'package:gui_flutter/library/providers.dart';
import 'package:gui_flutter/src/rust/api/fs_browser.dart';
import 'package:gui_flutter/library/drive_pane.dart';
import 'package:gui_flutter/shell/material_theme.dart';
import 'package:gui_flutter/shell/mixar_theme.dart';
import 'package:gui_flutter/library/library_nav.dart';
import 'package:gui_flutter/src/rust/api/library.dart';

import 'helpers/collection_tracks_override.dart';
import 'support/fake_library_transport.dart';
import 'support/mixar_material_app.dart';

/// Emits library evts the way the controller session does, so the tests run the
/// real transport → handler path instead of calling the dispatch directly.
void main() {
  const collectionA = LibraryCollectionSummary(
    id: 'c1',
    name: 'alpha',
    kind: 'folder',
    path: '/tmp/alpha',
    trackCount: 2,
  );
  const collectionB = LibraryCollectionSummary(
    id: 'c2',
    name: 'beta',
    kind: 'folder',
    path: '/tmp/beta',
    trackCount: 1,
  );
  final sessionA = HistorySessionSummary(
    id: 's1',
    title: 'Friday',
    startedAt: '2026-09-04T22:00:00Z',
    lastActivityAt: '2026-09-04T23:00:00Z',
    closed: true,
    entryCount: 3,
  );
  final sessionB = HistorySessionSummary(
    id: 's2',
    title: 'Saturday',
    startedAt: '2026-09-05T22:00:00Z',
    lastActivityAt: '2026-09-05T23:00:00Z',
    closed: true,
    entryCount: 5,
  );
  const folders = [
    FsEntry(name: 'a', path: '/tmp/samples/a'),
    FsEntry(name: 'b', path: '/tmp/samples/b'),
  ];
  final listing = const FsDirectoryListing(
    path: '/tmp/samples',
    parent: '/tmp',
    directories: folders,
    audioFiles: [],
  );
  const rootVolume = FsVolumeInfo(name: 'root', path: '/', isRemovable: false);
  const dataVolume = FsVolumeInfo(
    name: 'data',
    path: '/mnt/data',
    isRemovable: true,
  );

  final baseOverrides = [
    collectionsProvider.overrideWith((ref) async => [collectionA, collectionB]),
    overrideCollectionTracksValue(const []),
    historySessionsProvider.overrideWith((ref) async => [sessionA, sessionB]),
    driveVolumesProvider.overrideWith((ref) async => [rootVolume, dataVolume]),
  ];

  /// Pumps a host whose library transport replays the given evts.
  ///
  /// [loadListing] replaces the drive listing source, for a test that mutates
  /// the tree under an open browse cursor (it re-reads on refresh).
  Future<(ProviderContainer, void Function(LibraryEvt))> pumpBrowser(
    WidgetTester tester, {
    Future<FsDirectoryListing> Function(Ref ref)? loadListing,
  }) async {
    final transport = FakeLibraryTransport();
    addTearDown(transport.dispose);
    late ProviderContainer container;
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          ...baseOverrides,
          if (loadListing == null)
            driveListingProvider.overrideWith((ref) async => listing)
          else
            driveListingProvider.overrideWith(loadListing),
          libraryTransportProvider.overrideWith((ref) async => transport),
        ],
        child: Directionality(
          textDirection: TextDirection.ltr,
          child: Consumer(
            builder: (context, ref, _) {
              container = ProviderScope.containerOf(context);
              // The real panes watch these; standing in for them keeps the
              // sidebar browse from reading a listing nobody has loaded.
              ref.watch(collectionsProvider);
              ref.watch(historySessionsProvider);
              ref.watch(driveListingProvider);
              ref.watch(driveVolumesProvider);
              ref.watch(libraryEventsBootstrapProvider);
              return const Text('browser');
            },
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();
    return (container, (LibraryEvt evt) => transport.events.add(evt));
  }

  const focusNext = LibraryEvt(kind: LibraryEvtKind.focus, delta: 1);
  const focusPrev = LibraryEvt(kind: LibraryEvtKind.focus, delta: -1);
  const browseNext = LibraryEvt(kind: LibraryEvtKind.navigate, delta: 1);
  const browsePrev = LibraryEvt(kind: LibraryEvtKind.navigate, delta: -1);

  test('a focus step flips panes either way', () {
    final container = ProviderContainer();
    addTearDown(container.dispose);
    final zone = container.read(libraryFocusZoneProvider.notifier);
    // The track list starts focused: its rows are the default browse target.
    expect(
      container.read(libraryFocusZoneProvider),
      LibraryFocusZone.trackList,
    );

    zone.move(1);
    expect(container.read(libraryFocusZoneProvider), LibraryFocusZone.sidebar);
    zone.move(-1);
    expect(
      container.read(libraryFocusZoneProvider),
      LibraryFocusZone.trackList,
    );
    // One pane per step whatever the magnitude: a relative knob reporting
    // several ticks must not walk past the other side.
    zone.move(5);
    expect(container.read(libraryFocusZoneProvider), LibraryFocusZone.sidebar);
    zone.move(-9);
    expect(
      container.read(libraryFocusZoneProvider),
      LibraryFocusZone.trackList,
    );
    zone.move(-1);
    expect(container.read(libraryFocusZoneProvider), LibraryFocusZone.sidebar);
    // A step with no ticks is not a focus step.
    zone.move(0);
    expect(container.read(libraryFocusZoneProvider), LibraryFocusZone.sidebar);
  });

  testWidgets('a focus step hands the browser between the panes', (
    tester,
  ) async {
    final (container, emit) = await pumpBrowser(tester);

    emit(focusNext);
    await tester.pump();
    expect(container.read(libraryFocusZoneProvider), LibraryFocusZone.sidebar);

    emit(focusNext);
    await tester.pump();
    expect(
      container.read(libraryFocusZoneProvider),
      LibraryFocusZone.trackList,
    );

    emit(focusPrev);
    await tester.pump();
    expect(container.read(libraryFocusZoneProvider), LibraryFocusZone.sidebar);
  });

  testWidgets('browse moves the track row while the list owns focus', (
    tester,
  ) async {
    final (container, emit) = await pumpBrowser(tester);
    container.read(focusedTrackRowIndexProvider.notifier).setCount(5);
    container.read(focusedTrackRowIndexProvider.notifier).set(1);

    emit(browseNext);
    await tester.pump();
    expect(container.read(focusedTrackRowIndexProvider), 2);

    emit(browsePrev);
    await tester.pump();
    expect(container.read(focusedTrackRowIndexProvider), 1);
  });

  testWidgets('browse moves the sidebar row while the sidebar owns focus', (
    tester,
  ) async {
    final (container, emit) = await pumpBrowser(tester);
    emit(focusNext);
    await tester.pump();

    // Nothing selected yet: the first collection is the focused row, and a step
    // off the top clamps onto it rather than selecting nothing.
    expect(container.read(activeCollectionIdProvider), collectionA.id);
    emit(browsePrev);
    await tester.pump();
    expect(container.read(selectedCollectionIdProvider), collectionA.id);

    emit(browseNext);
    await tester.pump();
    expect(container.read(selectedCollectionIdProvider), collectionB.id);

    // The list end clamps: no wrap to the first collection.
    emit(browseNext);
    await tester.pump();
    expect(container.read(activeCollectionIdProvider), collectionB.id);
  });

  testWidgets('a stale sidebar selection still steps from the shown row', (
    tester,
  ) async {
    final (container, emit) = await pumpBrowser(tester);
    // A row that is no longer in the list: the provider falls back to the first
    // row, and the browse must step from that row rather than from nowhere.
    container.read(selectedCollectionIdProvider.notifier).set('deleted');
    expect(container.read(activeCollectionIdProvider), collectionA.id);
    emit(focusNext);
    await tester.pump();

    emit(browseNext);
    await tester.pump();
    expect(container.read(selectedCollectionIdProvider), collectionB.id);

    emit(browsePrev);
    await tester.pump();
    expect(container.read(selectedCollectionIdProvider), collectionA.id);
  });

  testWidgets('sidebar browse follows the active tab', (tester) async {
    final (container, emit) = await pumpBrowser(tester);
    container
        .read(librarySourceTabProvider.notifier)
        .set(LibrarySourceTab.history);
    emit(focusNext);
    await tester.pump();

    expect(container.read(activeHistorySessionIdProvider), sessionA.id);
    emit(browseNext);
    await tester.pump();
    expect(container.read(selectedHistorySessionIdProvider), sessionB.id);
  });

  testWidgets('drive browse walks the tree and opens the row it lands on', (
    tester,
  ) async {
    final (container, emit) = await pumpBrowser(tester);
    container.read(driveCurrentPathProvider.notifier).set(listing.path);
    container
        .read(librarySourceTabProvider.notifier)
        .set(LibrarySourceTab.drive);
    emit(focusNext);
    await tester.pumpAndSettle();

    // Nothing browsed yet, so the cursor reads as the first row — the "up" row.
    expect(container.read(driveTreeCursorProvider), isNull);
    expect(container.read(driveTreeFocusedPathProvider), listing.parent);

    emit(browseNext);
    await tester.pumpAndSettle();
    expect(container.read(driveTreeCursorProvider), folders[0].path);
    expect(container.read(driveCurrentPathProvider), folders[0].path);

    emit(browseNext);
    await tester.pumpAndSettle();
    expect(container.read(driveTreeCursorProvider), folders[1].path);
    expect(container.read(driveCurrentPathProvider), folders[1].path);

    // Stepping back walks out through the sibling and onto the "up" row, which
    // is what opens the parent.
    emit(browsePrev);
    await tester.pumpAndSettle();
    expect(container.read(driveTreeCursorProvider), folders[0].path);
    expect(container.read(driveCurrentPathProvider), folders[0].path);

    emit(browsePrev);
    await tester.pumpAndSettle();
    expect(container.read(driveTreeCursorProvider), listing.parent);
    expect(container.read(driveCurrentPathProvider), listing.parent);
  });

  testWidgets('drive browse opens the next volume when no folder is open', (
    tester,
  ) async {
    final (container, emit) = await pumpBrowser(tester);
    container
        .read(librarySourceTabProvider.notifier)
        .set(LibrarySourceTab.drive);
    emit(focusNext);
    await tester.pumpAndSettle();

    // No folder open: the tree is the volume list, starting on the first.
    expect(container.read(driveTreeCursorProvider), isNull);
    expect(container.read(driveTreeFocusedPathProvider), rootVolume.path);

    emit(browseNext);
    await tester.pumpAndSettle();
    expect(container.read(driveTreeCursorProvider), dataVolume.path);
    expect(container.read(driveCurrentPathProvider), dataVolume.path);
  });

  testWidgets('a listing that shrinks pulls the drive cursor back in range', (
    tester,
  ) async {
    // A folder that loses subfolders while the browser is open: the cursor sat
    // on the last row, so it must fall back onto a row that still exists
    // instead of stepping from an index past the end.
    var rows = listing;
    final (container, emit) = await pumpBrowser(
      tester,
      loadListing: (ref) async => rows,
    );
    container.read(driveCurrentPathProvider.notifier).set(rows.path);
    container
        .read(librarySourceTabProvider.notifier)
        .set(LibrarySourceTab.drive);
    emit(focusNext);
    await tester.pumpAndSettle();

    emit(browseNext);
    await tester.pumpAndSettle();
    emit(browseNext);
    await tester.pumpAndSettle();
    expect(container.read(driveTreeCursorProvider), folders[1].path);
    expect(container.read(driveCurrentPathProvider), folders[1].path);

    // The folder now holds one subfolder: rows are ["up", a].
    rows = FsDirectoryListing(
      path: listing.path,
      parent: listing.parent,
      directories: [folders[0]],
      audioFiles: [],
    );
    await container.refresh(driveListingProvider.future);
    await tester.pumpAndSettle();
    // The cursor's row is gone, so it reads as the first row rather than
    // pointing at a row that no longer exists.
    expect(container.read(driveTreeCursorProvider), folders[1].path);
    expect(container.read(driveTreeFocusedPathProvider), listing.parent);

    // The top of the tree clamps there rather than moving.
    emit(browsePrev);
    await tester.pumpAndSettle();
    expect(container.read(driveTreeCursorProvider), listing.parent);
    expect(container.read(driveCurrentPathProvider), listing.parent);
  });

  testWidgets('browsing after an externally opened folder starts from its '
      'first row', (tester) async {
    // A tree deep enough that the listing follows the open folder, the way the
    // real filesystem browser does.
    final subA = const FsEntry(name: 'a1', path: '/tmp/samples/a/a1');
    final subB = const FsEntry(name: 'b1', path: '/tmp/samples/b/b1');
    FsDirectoryListing listingFor(String path) {
      if (path == folders[0].path) {
        return FsDirectoryListing(
          path: folders[0].path,
          parent: listing.path,
          directories: [subA],
          audioFiles: const [],
        );
      }
      if (path == folders[1].path) {
        return FsDirectoryListing(
          path: folders[1].path,
          parent: listing.path,
          directories: [subB],
          audioFiles: const [],
        );
      }
      return listing;
    }

    final (container, emit) = await pumpBrowser(
      tester,
      loadListing: (ref) async =>
          listingFor(ref.watch(driveCurrentPathProvider) ?? listing.path),
    );
    container
        .read(librarySourceTabProvider.notifier)
        .set(LibrarySourceTab.drive);
    container.read(driveCurrentPathProvider.notifier).set(listing.path);
    await tester.pumpAndSettle();
    emit(focusNext);
    await tester.pumpAndSettle();

    // Walk deep so the cursor holds a row of this listing.
    emit(browseNext);
    await tester.pumpAndSettle();
    expect(container.read(driveTreeCursorProvider), folders[0].path);
    expect(container.read(driveCurrentPathProvider), folders[0].path);

    // A path change from outside the rotary (a row press, or "Browse in Drive"):
    // the cursor's row is not in the new listing, so it reads as that listing's
    // first row and the next step starts from there.
    container.read(driveCurrentPathProvider.notifier).set(folders[1].path);
    await tester.pumpAndSettle();
    // First row of the new listing: the "up" row, i.e. the folder just left.
    expect(container.read(driveTreeFocusedPathProvider), listing.path);
    emit(browseNext);
    await tester.pumpAndSettle();
    expect(container.read(driveTreeCursorProvider), subB.path);
    expect(container.read(driveCurrentPathProvider), subB.path);
  });
  testWidgets('an "up" row press parks focus and the cursor like any other '
      'row', (tester) async {
    final subA = const FsEntry(name: 'a1', path: '/tmp/samples/a/a1');
    FsDirectoryListing listingFor(String path) {
      if (path == folders[0].path) {
        return FsDirectoryListing(
          path: folders[0].path,
          parent: listing.path,
          directories: [subA],
          audioFiles: const [],
        );
      }
      return listing;
    }

    // The pane itself: the contract is about which row a press moves the cursor
    // onto, so the row has to be tappable.
    final theme = MixarThemeData.light();
    final transport = FakeLibraryTransport();
    addTearDown(transport.dispose);
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          ...baseOverrides,
          driveListingProvider.overrideWith(
            (ref) async =>
                listingFor(ref.watch(driveCurrentPathProvider) ?? listing.path),
          ),
          libraryTransportProvider.overrideWith((ref) async => transport),
        ],
        child: MaterialApp(
          theme: materialUiThemeFromMixar(theme),
          builder: mixarMaterialAppBuilder(theme),
          home: const Scaffold(
            // A size for the pane: an unbounded child lays the list out empty.
            body: SizedBox(width: 900, height: 600, child: DrivePane()),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();
    final container = ProviderScope.containerOf(
      tester.element(find.byType(DrivePane)),
    );
    container.read(driveCurrentPathProvider.notifier).set(listing.path);
    await tester.pumpAndSettle();
    // Park the cursor on a subfolder and hand the keys to the track list, so
    // the press below has to move both.
    container.read(driveTreeCursorProvider.notifier).set(folders[0].path);
    container
        .read(libraryFocusZoneProvider.notifier)
        .set(LibraryFocusZone.trackList);
    await tester.pump();

    // The "up" row: the open folder, shown by its own name. Matched through the
    // row so a volume or folder that happens to share the name cannot be hit.
    final upRow = find.ancestor(
      of: find.text('samples'),
      matching: find.byType(LibraryNavRow),
    );
    expect(upRow, findsOneWidget);
    await tester.tap(upRow.first);
    await tester.pumpAndSettle();

    expect(container.read(libraryFocusZoneProvider), LibraryFocusZone.sidebar);
    expect(container.read(driveCurrentPathProvider), listing.parent);
    expect(container.read(driveTreeCursorProvider), listing.parent);
  });
}
