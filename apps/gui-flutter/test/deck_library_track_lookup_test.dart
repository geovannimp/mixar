import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/library/providers.dart';
import 'package:gui_flutter/mixer/engine_providers.dart';
import 'package:gui_flutter/src/rust/api/library.dart';

void main() {
  test('libraryTrackById helper matches by id', () {
    const tracks = [
      LibraryTrackSummary(
        id: 'a',
        displayName: 'A',
        path: '/a.flac',
        key: '8A',
      ),
      LibraryTrackSummary(
        id: 'b',
        displayName: 'B',
        path: '/b.flac',
        key: '9B',
      ),
    ];
    expect(libraryTrackById(tracks, 'b')?.key, '9B');
    expect(libraryTrackById(tracks, 'missing'), isNull);
    expect(libraryTrackById(null, 'a'), isNull);
  });

  test('deckLibraryTrackProvider prefers visible table over getTrack', () {
    const table = LibraryTrackSummary(
      id: 'a',
      displayName: 'Table',
      path: '/a.flac',
      key: '8A',
    );
    const fetched = LibraryTrackSummary(
      id: 'a',
      displayName: 'Fetched',
      path: '/a.flac',
      key: '1A',
    );
    final container = ProviderContainer(
      overrides: [
        deckTrackIdProvider(0).overrideWith((ref) => 'a'),
        libraryTableTracksProvider.overrideWith(
          (ref) => const AsyncData([table]),
        ),
        collectionTracksProvider.overrideWith(
          (ref) async => const <LibraryTrackSummary>[],
        ),
        driveResolvedByPathProvider.overrideWith(
          (ref) async => const <String, LibraryTrackSummary>{},
        ),
        libraryTrackByIdProvider('a').overrideWith((ref) async => fetched),
      ],
    );
    addTearDown(container.dispose);

    expect(container.read(deckLibraryTrackProvider(0))?.key, '8A');
  });

  test(
    'deckLibraryTrackProvider falls back to libraryTrackByIdProvider',
    () async {
      const fetched = LibraryTrackSummary(
        id: 'a',
        displayName: 'Fetched',
        path: '/a.flac',
        key: '1A',
      );
      final container = ProviderContainer(
        overrides: [
          deckTrackIdProvider(0).overrideWith((ref) => 'a'),
          libraryTableTracksProvider.overrideWith(
            (ref) => const AsyncData(<LibraryTrackSummary>[]),
          ),
          collectionTracksProvider.overrideWith(
            (ref) async => const <LibraryTrackSummary>[],
          ),
          driveResolvedByPathProvider.overrideWith(
            (ref) async => const <String, LibraryTrackSummary>{},
          ),
          libraryTrackByIdProvider('a').overrideWith((ref) async => fetched),
        ],
      );
      addTearDown(container.dispose);

      expect(container.read(deckLibraryTrackProvider(0)), isNull);
      await container.read(libraryTrackByIdProvider('a').future);
      expect(container.read(deckLibraryTrackProvider(0))?.key, '1A');
    },
  );
}
