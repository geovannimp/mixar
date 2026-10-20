import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/providers/library/providers.dart';
import 'package:gui_flutter/src/rust/api/library.dart';

import 'helpers/collection_tracks_override.dart';

LibraryTrackSummary _track({
  required String id,
  required String displayName,
  String? title,
  String? artist,
  String? comment,
  LibraryColorLabel? colorLabel,
  int? rating,
  int? year,
}) {
  return LibraryTrackSummary(
    id: id,
    displayName: displayName,
    artist: artist,
    title: title,
    comment: comment,
    colorLabel: colorLabel,
    rating: rating,
    year: year,
    durationMs: 1000,
    path: '/tmp/$id.wav',
  );
}

void main() {
  late ProviderContainer container;
  final tracks = [
    _track(
      id: '1',
      displayName: 'palawan',
      title: 'Palawan by SKIRK Vlog Music [xXRDR-ycleo]',
    ),
    _track(
      id: '2',
      displayName: 'elegy',
      title: 'Z8phyR - Nameless Elegy (Second Mix)',
    ),
    _track(
      id: '3',
      displayName: 'stem-name',
      title: '',
      artist: 'Solo Artist',
      comment: 'Late-night warmup',
      colorLabel: LibraryColorLabel.aqua,
      rating: 2,
      year: 1999,
    ),
  ];

  setUp(() async {
    container = ProviderContainer(
      overrides: [
        collectionsProvider.overrideWith(
          (ref) async => [
            const LibraryCollectionSummary(
              id: 'col-1',
              name: 'Collection',
              kind: 'playlist',
              trackCount: 3,
            ),
          ],
        ),
        overrideCollectionTracksValue(tracks),
      ],
    );
    addTearDown(container.dispose);
    await container.read(collectionsProvider.future);
    await container.read(collectionTracksProvider.future);
  });

  List<String> filteredIds() {
    return container
        .read(filteredTracksProvider)
        .requireValue
        .map((t) => t.id)
        .toList();
  }

  void expectFilterMatches(String query, List<String> ids, {String? reason}) {
    container.read(trackFilterProvider.notifier).set(query);
    expect(filteredIds(), ids, reason: reason);
  }

  test('filteredTracksProvider matches mid-title Vlog', () {
    expectFilterMatches('Vlog', ['1']);
  });

  test('filteredTracksProvider matches mid-title SKIRK', () {
    expectFilterMatches('SKIRK', ['1']);
  });

  test('filteredTracksProvider matches artist', () {
    expectFilterMatches('Solo', ['3']);
  });

  test('filteredTracksProvider uses displayName when title is empty', () {
    expectFilterMatches('stem-name', ['3']);
  });

  test('filteredTracksProvider searches comment, colour and year', () {
    expectFilterMatches('warmup', ['3'], reason: 'comment');
    expectFilterMatches('aqua', ['3'], reason: 'colour label');
    expectFilterMatches('1999', ['3'], reason: 'year');
  });

  test('unlabelled tracks do not match the None placeholder', () {
    expectFilterMatches('none', const []);
  });

  test(
    'sortLibraryTracks orders colours by the palette, not enum spelling',
    () {
      final colored = [
        _track(
          id: 'purple',
          displayName: 'purple',
          colorLabel: LibraryColorLabel.purple,
        ),
        _track(
          id: 'pink',
          displayName: 'pink',
          colorLabel: LibraryColorLabel.pink,
        ),
        _track(
          id: 'aqua',
          displayName: 'aqua',
          colorLabel: LibraryColorLabel.aqua,
        ),
      ];
      final sorted = sortLibraryTracks(
        colored,
        LibrarySortField.color,
        ascending: true,
      );
      expect(sorted.map((t) => t.id).toList(), ['pink', 'aqua', 'purple']);
    },
  );

  test('sortLibraryTracks orders ratings and puts unrated tracks last', () {
    final sorted = sortLibraryTracks(
      tracks,
      LibrarySortField.rating,
      ascending: false,
    );
    expect(sorted.map((t) => t.id).toList(), ['3', '1', '2']);
  });
}
