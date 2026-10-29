import 'dart:async';

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/providers/library/providers.dart';
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
  String kind = 'playlist',
}) {
  return LibraryCollectionSummary(
    id: id,
    name: 'Collection',
    kind: kind,
    trackCount: kind == 'smart_playlist' ? null : trackCount,
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

      // Resolve collections first so SeededCollectionTracks' active-id watch
      // does not rebuild the seed after the first track load.
      await container.read(collectionsProvider.future);
      await container.read(collectionTracksProvider.future);
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

        await container.read(collectionsProvider.future);
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

        await container.read(collectionsProvider.future);
        await container.read(collectionTracksProvider.future);
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

        await container.read(collectionsProvider.future);
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

      await container.read(collectionsProvider.future);
      await container.read(collectionTracksProvider.future);
      expect(trackLoads, 1);
      expect(collectionLoads, 1);

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

        await container.read(collectionsProvider.future);
        await container.read(collectionTracksProvider.future);
        expect(trackLoads, 1);
        expect(collectionLoads, 1);

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

      await container.read(collectionsProvider.future);
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

        await container.read(collectionsProvider.future);
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
      'metadata trackUpdated during AsyncLoading is dropped without thrash',
      () async {
        var trackLoads = 0;
        final gate = Completer<List<LibraryTrackSummary>>();
        final seed = _track(id: 'a', bpm: 120);
        final container = ProviderContainer(
          overrides: [
            collectionsProvider.overrideWith((ref) async => [_collection()]),
            overrideCollectionTracks((ref) async {
              trackLoads++;
              return gate.future;
            }),
          ],
        );
        addTearDown(container.dispose);

        await container.read(collectionsProvider.future);
        final load = container.read(collectionTracksProvider.future);
        await pumpEventQueue();
        expect(trackLoads, 1);
        expect(container.read(collectionTracksProvider).isLoading, isTrue);

        container.read(libraryEvtDispatchProvider)(
          LibraryEvt(
            kind: LibraryEvtKind.trackUpdated,
            track: _track(id: 'a', bpm: 128),
          ),
        );
        await pumpEventQueue();

        // Dropped while loading: no invalidate/reload thrash.
        expect(trackLoads, 1);
        expect(container.read(collectionTracksProvider).isLoading, isTrue);

        gate.complete([seed]);
        await load;
        expect(trackLoads, 1);
        expect(
          container.read(collectionTracksProvider).requireValue.single.bpm,
          120,
        );
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
            .select(LibrarySortField.bpm);

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

    test(
      'seeded override rebuilds when the active collection changes',
      () async {
        var trackLoads = 0;
        final container = ProviderContainer(
          overrides: [
            collectionsProvider.overrideWith(
              (ref) async => [
                _collection(id: 'col-1'),
                _collection(id: 'col-2'),
              ],
            ),
            overrideCollectionTracks((ref) async {
              trackLoads++;
              final id = ref.watch(activeCollectionIdProvider);
              return [_track(id: id ?? 'none')];
            }),
          ],
        );
        addTearDown(container.dispose);

        await container.read(collectionsProvider.future);
        await container.read(collectionTracksProvider.future);
        expect(trackLoads, 1);
        expect(
          container.read(collectionTracksProvider).requireValue.single.id,
          'col-1',
        );

        container.read(selectedCollectionIdProvider.notifier).set('col-2');
        await container.read(collectionTracksProvider.future);
        expect(trackLoads, 2);
        expect(
          container.read(collectionTracksProvider).requireValue.single.id,
          'col-2',
        );
      },
    );

    test(
      'drops a pending patch burst when the active collection changes',
      () async {
        var trackLoads = 0;
        final container = ProviderContainer(
          overrides: [
            collectionsProvider.overrideWith(
              (ref) async => [
                _collection(id: 'col-1'),
                _collection(id: 'col-2'),
              ],
            ),
            overrideCollectionTracks((ref) async {
              trackLoads++;
              final id = ref.watch(activeCollectionIdProvider);
              return [_track(id: id == 'col-2' ? 'b' : 'a', bpm: 120)];
            }),
          ],
        );
        addTearDown(container.dispose);

        await container.read(collectionsProvider.future);
        await container.read(collectionTracksProvider.future);
        expect(trackLoads, 1);

        container.read(libraryEvtDispatchProvider)(
          LibraryEvt(
            kind: LibraryEvtKind.trackUpdated,
            track: _track(id: 'a', bpm: 128),
          ),
        );
        // Switch before the coalesced microtask flush runs.
        container.read(selectedCollectionIdProvider.notifier).set('col-2');
        await pumpEventQueue();
        await container.read(collectionTracksProvider.future);

        expect(trackLoads, 2);
        expect(
          container.read(collectionTracksProvider).requireValue.single.id,
          'b',
        );
        expect(
          container.read(collectionTracksProvider).requireValue.single.bpm,
          120,
        );
      },
    );

    test('rebuild clears a pending patch so the fresh load wins', () async {
      var trackLoads = 0;
      final container = ProviderContainer(
        overrides: [
          collectionsProvider.overrideWith((ref) async => [_collection()]),
          overrideCollectionTracks((ref) async {
            trackLoads++;
            // Second load simulates SQLite already written to 140.
            return [_track(id: 'a', bpm: trackLoads == 1 ? 120 : 140)];
          }),
        ],
      );
      addTearDown(container.dispose);

      await container.read(collectionsProvider.future);
      await container.read(collectionTracksProvider.future);
      expect(trackLoads, 1);

      container.read(libraryEvtDispatchProvider)(
        LibraryEvt(
          kind: LibraryEvtKind.trackUpdated,
          track: _track(id: 'a', bpm: 128),
        ),
      );
      // Same-collection invalidate before the coalesced flush (e.g. detail
      // dialog save). Pending burst must not overwrite the reloaded row.
      container.invalidate(collectionTracksProvider);
      await pumpEventQueue();
      await container.read(collectionTracksProvider.future);

      expect(trackLoads, 2);
      expect(
        container.read(collectionTracksProvider).requireValue.single.bpm,
        140,
      );
    });

    test(
      'scheduled flush is a no-op after the container is disposed',
      () async {
        final seed = _track(id: 'a', bpm: 120);
        final container = ProviderContainer(
          overrides: [
            collectionsProvider.overrideWith((ref) async => [_collection()]),
            overrideCollectionTracksValue([seed]),
          ],
        );
        var disposed = false;
        addTearDown(() {
          if (!disposed) {
            container.dispose();
          }
        });

        await container.read(collectionsProvider.future);
        await container.read(collectionTracksProvider.future);
        container.read(libraryEvtDispatchProvider)(
          LibraryEvt(
            kind: LibraryEvtKind.trackUpdated,
            track: _track(id: 'a', bpm: 128),
          ),
        );
        disposed = true;
        container.dispose();
        await pumpEventQueue();
      },
    );
  });

  group('smart playlists', () {
    test('a row event reloads the list without recounting the sidebar', () async {
      var trackLoads = 0;
      var collectionLoads = 0;
      final container = ProviderContainer(
        overrides: [
          collectionsProvider.overrideWith((ref) async {
            collectionLoads++;
            return [_collection(kind: 'smart_playlist')];
          }),
          overrideCollectionTracks((ref) async {
            trackLoads++;
            return [_track(id: 'a')];
          }),
        ],
      );
      addTearDown(container.dispose);
      final sub = container.listen(collectionTracksProvider, (_, _) {});
      addTearDown(sub.close);
      await container.read(collectionsProvider.future);
      expect(
        await container.read(collectionTracksProvider.future),
        hasLength(1),
      );
      // The seeded list loads once for the null active id and once when the
      // collections resolve; both are startup noise, not event work.
      final trackLoadsBefore = trackLoads;
      final collectionLoadsBefore = collectionLoads;

      container
          .read(collectionTracksProvider.notifier)
          .scheduleTrackRowEvent(_track(id: 'a'));
      await pumpEventQueue();

      // Rules decide membership, so the row cannot be patched in place...
      expect(trackLoads, trackLoadsBefore + 1);
      // ...but a smart playlist stores no count, so the sidebar cannot change.
      expect(collectionLoads, collectionLoadsBefore);
    });

    test('a row event during a load does not restart the query', () async {
      var trackLoads = 0;
      final gate = Completer<List<LibraryTrackSummary>>();
      final container = ProviderContainer(
        overrides: [
          collectionsProvider.overrideWith(
            (ref) async => [_collection(kind: 'smart_playlist')],
          ),
          overrideCollectionTracks((ref) {
            trackLoads++;
            return gate.future;
          }),
        ],
      );
      addTearDown(container.dispose);
      final sub = container.listen(collectionTracksProvider, (_, _) {});
      addTearDown(sub.close);
      await container.read(collectionsProvider.future);
      await pumpEventQueue();
      final loadsBefore = trackLoads;

      // The load is still in flight; a chatty analysis burst must not queue
      // another one behind it.
      container
          .read(collectionTracksProvider.notifier)
          .scheduleTrackRowEvent(_track(id: 'a'));
      container
          .read(collectionTracksProvider.notifier)
          .scheduleTrackRowEvent(null);
      await pumpEventQueue();
      expect(trackLoads, loadsBefore, reason: 'no restart while loading');

      gate.complete([_track(id: 'a')]);
      await container.read(collectionTracksProvider.future);
      expect(trackLoads, loadsBefore);
    });
  });
}
