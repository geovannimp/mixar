import 'dart:async';

import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/library/history_providers.dart';
import 'package:gui_flutter/library/providers.dart';
import 'package:gui_flutter/src/rust/api/fs_browser.dart';
import 'package:gui_flutter/src/rust/api/library.dart';

import 'helpers/collection_tracks_override.dart';
import 'support/fake_library_transport.dart';

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

  /// The drive listing override, held so a pump can swap it out.
  final baseListingOverride = driveListingProvider.overrideWith(
    (ref) async => listing,
  );

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
          ...baseOverrides.where((o) => !identical(o, baseListingOverride)),
          if (loadListing == null)
            baseListingOverride
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

    // The cursor is an offset into the visible rows, and the "up" row is row 0,
    // so it starts on the parent.
    expect(container.read(driveTreeCursorProvider), 0);
    expect(container.read(driveTreeFocusedPathProvider), listing.parent);

    emit(browseNext);
    await tester.pumpAndSettle();
    expect(container.read(driveTreeCursorProvider), 1);
    expect(container.read(driveCurrentPathProvider), folders[0].path);

    emit(browseNext);
    await tester.pumpAndSettle();
    expect(container.read(driveTreeCursorProvider), 2);
    expect(container.read(driveCurrentPathProvider), folders[1].path);

    // Stepping back walks out through the sibling and onto the "up" row, which
    // is what opens the parent.
    emit(browsePrev);
    await tester.pumpAndSettle();
    expect(container.read(driveTreeCursorProvider), 1);
    expect(container.read(driveCurrentPathProvider), folders[0].path);

    emit(browsePrev);
    await tester.pumpAndSettle();
    expect(container.read(driveTreeCursorProvider), 0);
    expect(container.read(driveTreeFocusedPathProvider), listing.parent);
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

    // No folder open: the tree is the volume list.
    expect(container.read(driveTreeCursorProvider), 0);
    expect(container.read(driveTreeFocusedPathProvider), rootVolume.path);

    emit(browseNext);
    await tester.pumpAndSettle();
    expect(container.read(driveTreeCursorProvider), 1);
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
    expect(container.read(driveTreeCursorProvider), 2);
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
    expect(container.read(driveTreeFocusedPathProvider), folders[0].path);

    // One step back moves the cursor rather than clamping in place.
    emit(browsePrev);
    await tester.pumpAndSettle();
    expect(container.read(driveTreeCursorProvider), 0);
    expect(container.read(driveCurrentPathProvider), listing.parent);
  });
}
