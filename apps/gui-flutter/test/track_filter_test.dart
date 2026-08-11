import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/library/providers.dart';
import 'package:gui_flutter/src/rust/api/library.dart';

LibraryTrackSummary _track({
  required String id,
  required String title,
  String? artist,
}) {
  return LibraryTrackSummary(
    id: id,
    displayName: title,
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
  test('trackTitleLabel falls back when title is empty', () {
    final t = LibraryTrackSummary(
      id: '1',
      displayName: 'stem-name',
      artist: null,
      title: '',
      album: null,
      genre: null,
      bpm: null,
      key: null,
      durationMs: 1,
      path: '/tmp/1.wav',
    );
    expect(trackTitleLabel(t), 'stem-name');
  });

  test('mid-title substring would match via contains', () {
    final title = 'Palawan by SKIRK Vlog Music [xXRDR-ycleo]';
    expect(title.toLowerCase().contains('vlog'), isTrue);
    expect(trackTitleLabel(_track(id: '1', title: title)).toLowerCase(),
        contains('skirk'));
  });
}
