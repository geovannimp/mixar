import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/library/providers.dart';
import 'package:gui_flutter/src/rust/api/library.dart';

import 'support/fake_library_transport.dart';

const _collection = LibraryCollectionSummary(
  id: 'c1',
  name: 'Library',
  kind: 'playlist',
  trackCount: 1,
);

const _track = LibraryTrackSummary(
  id: 't1',
  displayName: 'Track',
  title: 'Track',
  path: '/t1.flac',
);

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  Future<ProviderContainer> boot(FakeLibraryTransport fake) async {
    final container = ProviderContainer(
      overrides: [libraryTransportProvider.overrideWith((ref) async => fake)],
    );
    addTearDown(() {
      container.dispose();
      fake.dispose();
    });

    await container.read(libraryTransportProvider.future);
    // Bootstrap only attaches subscribeEvents once transport is AsyncData.
    // Provider<void> is sync: listening after invalidate means the subscription
    // is live before we emit events below.
    container.invalidate(libraryEventsBootstrapProvider);
    container.listen(libraryEventsBootstrapProvider, (_, __) {});
    // Touch the provider so a silent build failure would surface here.
    container.read(libraryEventsBootstrapProvider);

    await container.read(collectionsProvider.future);
    expect(container.read(activeCollectionIdProvider), _collection.id);

    container.listen(collectionTracksProvider, (_, __) {});
    await container.read(collectionTracksProvider.future);
    expect(fake.listCollectionEntriesCalls, isNotEmpty);
    return container;
  }

  test('trackUpdated invalidates collectionTracks (full re-list)', () async {
    final fake = FakeLibraryTransport(
      collections: const [_collection],
      entries: const [_track],
    );
    final container = await boot(fake);

    final before = fake.listCollectionEntriesCalls.length;

    fake.events.add(
      const LibraryEvt(
        kind: LibraryEvtKind.trackUpdated,
        trackId: 't1',
        track: _track,
      ),
    );
    await pumpEventQueue();
    await container.read(collectionTracksProvider.future);

    expect(fake.listCollectionEntriesCalls.length, before + 1);
  });

  test(
    'trackAnalyzed invalidates collectionTracks like trackUpdated',
    () async {
      final fake = FakeLibraryTransport(
        collections: const [_collection],
        entries: const [_track],
      );
      final container = await boot(fake);

      final before = fake.listCollectionEntriesCalls.length;
      fake.events.add(
        const LibraryEvt(
          kind: LibraryEvtKind.trackAnalyzed,
          trackId: 't1',
          track: _track,
        ),
      );
      await pumpEventQueue();
      await container.read(collectionTracksProvider.future);

      expect(fake.listCollectionEntriesCalls.length, before + 1);
    },
  );

  // Characterization: current handler has no debounce/coalesce and ignores
  // evt.track payloads (full re-list only). Update this assertion when #463
  // lands patch/coalesce behavior.
  test('burst of trackUpdated events does not coalesce list reloads', () async {
    final fake = FakeLibraryTransport(
      collections: const [_collection],
      entries: const [_track],
    );
    final container = await boot(fake);

    final before = fake.listCollectionEntriesCalls.length;

    // Space events so each invalidate can complete a reload.
    fake.events.add(
      const LibraryEvt(kind: LibraryEvtKind.trackUpdated, trackId: 't1'),
    );
    await pumpEventQueue();
    await container.read(collectionTracksProvider.future);

    fake.events.add(
      const LibraryEvt(
        kind: LibraryEvtKind.trackUpdated,
        trackId: 't1',
        track: LibraryTrackSummary(
          id: 't1',
          displayName: 'Track',
          title: 'Patched Title',
          path: '/t1.flac',
        ),
      ),
    );
    await pumpEventQueue();
    await container.read(collectionTracksProvider.future);

    expect(fake.listCollectionEntriesCalls.length, before + 2);
    final tracks = await container.read(collectionTracksProvider.future);
    // Payload on the event is ignored today — list re-fetch keeps fake.entries.
    expect(tracks.single.title, 'Track');
  });

  test(
    'trackUpdated reload failure surfaces on collectionTracksProvider',
    () async {
      var shouldFail = false;
      final fake = FakeLibraryTransport(
        collections: const [_collection],
        entries: const [_track],
        listCollectionEntriesHandler: (id) async {
          if (shouldFail) {
            throw StateError('list failed');
          }
          return const [_track];
        },
      );
      final container = await boot(fake);

      shouldFail = true;
      fake.events.add(
        const LibraryEvt(kind: LibraryEvtKind.trackUpdated, trackId: 't1'),
      );
      await pumpEventQueue();

      await expectLater(
        container.read(collectionTracksProvider.future),
        throwsA(isA<StateError>()),
      );
      expect(container.read(collectionTracksProvider).hasError, isTrue);
    },
  );
}
