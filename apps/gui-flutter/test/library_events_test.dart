import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/library/providers.dart';
import 'package:gui_flutter/src/rust/api/library.dart';

import 'helpers/collection_tracks_override.dart';

LibraryTrackSummary _track({
  required String id,
  String displayName = 'Track',
  double? bpm,
  String? key,
}) {
  return LibraryTrackSummary(
    id: id,
    displayName: displayName,
    bpm: bpm,
    key: key,
    durationMs: 180000,
    path: '/tmp/$id.flac',
  );
}

LibraryCollectionSummary _collection({
  String id = 'col-1',
  int trackCount = 1,
}) {
  return LibraryCollectionSummary(
    id: id,
    name: 'Collection',
    kind: 'playlist',
    trackCount: trackCount,
  );
}

void main() {
  group('library events — track list refresh (characterization #460)', () {
    test('trackUpdated with track present used to invalidate both providers; '
        'now patches without reload (#463)', () async {
      var trackLoads = 0;
      var collectionLoads = 0;
      final seed = _track(id: 'a', bpm: 120);
      final updated = _track(id: 'a', bpm: 128);
      final container = ProviderContainer(
        overrides: [
          collectionsProvider.overrideWith((ref) async {
            collectionLoads++;
            return [_collection()];
          }),
          overrideCollectionTracks((ref) async {
            trackLoads++;
            return [seed];
          }),
        ],
      );
      addTearDown(container.dispose);

      await container.read(collectionTracksProvider.future);
      await container.read(collectionsProvider.future);
      expect(trackLoads, 1);
      expect(collectionLoads, 1);

      container.read(libraryEvtDispatchProvider)(
        LibraryEvt(kind: LibraryEvtKind.trackUpdated, track: updated),
      );
      await pumpEventQueue();

      expect(trackLoads, 1);
      expect(collectionLoads, 1);
      expect(
        container.read(collectionTracksProvider).requireValue.single.bpm,
        128,
      );
    });

    test(
      'trackAnalyzed clears analyzing even when the row is patched in place',
      () async {
        var trackLoads = 0;
        final seed = _track(id: 'a', bpm: 120);
        final analyzed = _track(id: 'a', bpm: 128, key: '8A');
        final container = ProviderContainer(
          overrides: [
            collectionsProvider.overrideWith((ref) async => [_collection()]),
            overrideCollectionTracks((ref) async {
              trackLoads++;
              return [seed];
            }),
          ],
        );
        addTearDown(container.dispose);

        await container.read(collectionTracksProvider.future);
        container.read(analyzingTrackIdsProvider.notifier).add('a');

        container.read(libraryEvtDispatchProvider)(
          LibraryEvt(kind: LibraryEvtKind.trackAnalyzed, track: analyzed),
        );
        await pumpEventQueue();

        expect(trackLoads, 1);
        expect(container.read(analyzingTrackIdsProvider), isEmpty);
        expect(
          container.read(collectionTracksProvider).requireValue.single.key,
          '8A',
        );
      },
    );
  });

  group('library events — trackUpdated patch policy (#463)', () {
    test(
      'patches in-memory row from evt.track without reloading lists',
      () async {
        var trackLoads = 0;
        var collectionLoads = 0;
        final seed = _track(id: 'a', bpm: 120, key: '1A');
        final updated = _track(id: 'a', bpm: 128, key: '8A');
        final container = ProviderContainer(
          overrides: [
            collectionsProvider.overrideWith((ref) async {
              collectionLoads++;
              return [_collection()];
            }),
            overrideCollectionTracks((ref) async {
              trackLoads++;
              return [seed];
            }),
          ],
        );
        addTearDown(container.dispose);

        await container.read(collectionTracksProvider.future);
        await container.read(collectionsProvider.future);
        expect(trackLoads, 1);
        expect(collectionLoads, 1);

        container.read(libraryEvtDispatchProvider)(
          LibraryEvt(kind: LibraryEvtKind.trackUpdated, track: updated),
        );
        await pumpEventQueue();

        expect(trackLoads, 1);
        expect(collectionLoads, 1);
        final row = container
            .read(collectionTracksProvider)
            .requireValue
            .single;
        expect(row.bpm, 128);
        expect(row.key, '8A');
      },
    );

    test(
      'trackAnalyzed patches row and still clears analyzing without reload',
      () async {
        var trackLoads = 0;
        final seed = _track(id: 'a', bpm: 120);
        final analyzed = _track(id: 'a', bpm: 140, key: '9B');
        final container = ProviderContainer(
          overrides: [
            collectionsProvider.overrideWith((ref) async => [_collection()]),
            overrideCollectionTracks((ref) async {
              trackLoads++;
              return [seed];
            }),
          ],
        );
        addTearDown(container.dispose);

        await container.read(collectionTracksProvider.future);
        container.read(analyzingTrackIdsProvider.notifier).add('a');

        container.read(libraryEvtDispatchProvider)(
          LibraryEvt(kind: LibraryEvtKind.trackAnalyzed, track: analyzed),
        );
        await pumpEventQueue();

        expect(trackLoads, 1);
        expect(container.read(analyzingTrackIdsProvider), isEmpty);
        expect(
          container.read(collectionTracksProvider).requireValue.single.bpm,
          140,
        );
      },
    );

    test('full invalidate when evt.track is absent', () async {
      var trackLoads = 0;
      var collectionLoads = 0;
      final seed = _track(id: 'a', bpm: 120);
      final container = ProviderContainer(
        overrides: [
          collectionsProvider.overrideWith((ref) async {
            collectionLoads++;
            return [_collection()];
          }),
          overrideCollectionTracks((ref) async {
            trackLoads++;
            return [seed];
          }),
        ],
      );
      addTearDown(container.dispose);

      await container.read(collectionTracksProvider.future);
      await container.read(collectionsProvider.future);

      container.read(libraryEvtDispatchProvider)(
        const LibraryEvt(kind: LibraryEvtKind.trackUpdated, trackId: 'a'),
      );
      await pumpEventQueue();

      await container.read(collectionTracksProvider.future);
      await container.read(collectionsProvider.future);
      expect(trackLoads, 2);
      expect(collectionLoads, 2);
    });

    test(
      'full invalidate when track is not in the current collection list',
      () async {
        var trackLoads = 0;
        var collectionLoads = 0;
        final seed = _track(id: 'a', bpm: 120);
        final other = _track(id: 'b', bpm: 100);
        final container = ProviderContainer(
          overrides: [
            collectionsProvider.overrideWith((ref) async {
              collectionLoads++;
              return [_collection()];
            }),
            overrideCollectionTracks((ref) async {
              trackLoads++;
              return [seed];
            }),
          ],
        );
        addTearDown(container.dispose);

        await container.read(collectionTracksProvider.future);
        await container.read(collectionsProvider.future);

        container.read(libraryEvtDispatchProvider)(
          LibraryEvt(kind: LibraryEvtKind.trackUpdated, track: other),
        );
        await pumpEventQueue();

        await container.read(collectionTracksProvider.future);
        await container.read(collectionsProvider.future);
        expect(trackLoads, 2);
        expect(collectionLoads, 2);
      },
    );

    test('coalesces a burst of trackUpdated into one patch', () async {
      var trackLoads = 0;
      final seed = _track(id: 'a', bpm: 120);
      final container = ProviderContainer(
        overrides: [
          collectionsProvider.overrideWith((ref) async => [_collection()]),
          overrideCollectionTracks((ref) async {
            trackLoads++;
            return [seed];
          }),
        ],
      );
      addTearDown(container.dispose);

      await container.read(collectionTracksProvider.future);
      final dispatch = container.read(libraryEvtDispatchProvider);
      dispatch(
        LibraryEvt(
          kind: LibraryEvtKind.trackUpdated,
          track: _track(id: 'a', bpm: 121),
        ),
      );
      dispatch(
        LibraryEvt(
          kind: LibraryEvtKind.trackUpdated,
          track: _track(id: 'a', bpm: 122),
        ),
      );
      dispatch(
        LibraryEvt(
          kind: LibraryEvtKind.trackUpdated,
          track: _track(id: 'a', bpm: 123),
        ),
      );
      await pumpEventQueue();

      expect(trackLoads, 1);
      expect(
        container.read(collectionTracksProvider).requireValue.single.bpm,
        123,
      );
    });

    test(
      'metadata trackUpdated does not thrash reload while list is AsyncError',
      () async {
        var trackLoads = 0;
        final container = ProviderContainer(
          overrides: [
            collectionsProvider.overrideWith((ref) async => [_collection()]),
            overrideCollectionTracks((ref) async {
              trackLoads++;
              throw StateError('db down');
            }),
          ],
        );
        addTearDown(container.dispose);

        await expectLater(
          container.read(collectionTracksProvider.future),
          throwsA(isA<StateError>()),
        );
        expect(trackLoads, 1);
        expect(container.read(collectionTracksProvider).hasError, isTrue);

        container.read(libraryEvtDispatchProvider)(
          LibraryEvt(
            kind: LibraryEvtKind.trackUpdated,
            track: _track(id: 'a', bpm: 128),
          ),
        );
        await pumpEventQueue();

        expect(trackLoads, 1);
        expect(container.read(collectionTracksProvider).hasError, isTrue);
      },
    );

    test(
      'BPM-changing trackUpdated reorders libraryTableTracks via sort',
      () async {
        final seed = [
          _track(id: 'a', bpm: 100),
          _track(id: 'b', bpm: 120),
          _track(id: 'c', bpm: 140),
        ];
        final container = ProviderContainer(
          overrides: [
            collectionsProvider.overrideWith(
              (ref) async => [_collection(trackCount: 3)],
            ),
            overrideCollectionTracksValue(seed),
          ],
        );
        addTearDown(container.dispose);

        await container.read(collectionsProvider.future);
        await container.read(collectionTracksProvider.future);
        container
            .read(librarySortProvider.notifier)
            .setField(LibrarySortField.bpm);

        expect(
          container
              .read(libraryTableTracksProvider)
              .requireValue
              .map((t) => t.id)
              .toList(),
          ['a', 'b', 'c'],
        );

        container.read(libraryEvtDispatchProvider)(
          LibraryEvt(
            kind: LibraryEvtKind.trackUpdated,
            track: _track(id: 'c', bpm: 90),
          ),
        );
        await pumpEventQueue();

        // Provider keeps membership order; the table provider re-sorts.
        expect(
          container
              .read(collectionTracksProvider)
              .requireValue
              .map((t) => t.id)
              .toList(),
          ['a', 'b', 'c'],
        );
        expect(
          container
              .read(libraryTableTracksProvider)
              .requireValue
              .map((t) => t.id)
              .toList(),
          ['c', 'a', 'b'],
        );
      },
    );
  });
}
