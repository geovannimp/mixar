import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/library/providers.dart';
import 'package:gui_flutter/src/rust/api/library.dart';

LibraryTrackSummary _track({
  required String id,
  required String displayName,
  String? title,
  String? artist,
}) {
  return LibraryTrackSummary(
    id: id,
    displayName: displayName,
    artist: artist,
    title: title,
    album: null,
    genre: null,
    bpm: null,
    key: null,
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
    _track(id: '3', displayName: 'stem-name', title: '', artist: 'Solo Artist'),
  ];

  setUp(() async {
    container = ProviderContainer(
      overrides: [collectionTracksProvider.overrideWith((ref) async => tracks)],
    );
    addTearDown(container.dispose);
    await container.read(collectionTracksProvider.future);
  });

  List<String> filteredIds() {
    return container
        .read(filteredTracksProvider)
        .requireValue
        .map((t) => t.id)
        .toList();
  }

  test('filteredTracksProvider matches mid-title Vlog', () {
    container.read(trackFilterProvider.notifier).set('Vlog');
    expect(filteredIds(), ['1']);
  });

  test('filteredTracksProvider matches mid-title SKIRK', () {
    container.read(trackFilterProvider.notifier).set('SKIRK');
    expect(filteredIds(), ['1']);
  });

  test('filteredTracksProvider matches artist', () {
    container.read(trackFilterProvider.notifier).set('Solo');
    expect(filteredIds(), ['3']);
  });

  test('filteredTracksProvider uses displayName when title is empty', () {
    container.read(trackFilterProvider.notifier).set('stem-name');
    expect(filteredIds(), ['3']);
  });
}
