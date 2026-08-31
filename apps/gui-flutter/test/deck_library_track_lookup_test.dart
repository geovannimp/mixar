import 'package:flutter_test/flutter_test.dart';
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
}
