import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/utils/track_drag.dart';
import 'package:gui_flutter/providers/library/focused_load.dart';
import 'package:gui_flutter/src/rust/api/library.dart';

LibraryTrackSummary _track({
  required String id,
  required String path,
  String? title,
  String displayName = 'file.wav',
}) {
  return LibraryTrackSummary(
    id: id,
    displayName: displayName,
    title: title,
    path: path,
  );
}

void main() {
  group('filterAudioFilePaths', () {
    test('keeps supported extensions case-insensitively', () {
      expect(
        filterAudioFilePaths([
          '/music/a.mp3',
          '/music/readme.txt',
          '/music/b.FLAC',
          '/music/c.wav',
        ]),
        ['/music/a.mp3', '/music/b.FLAC', '/music/c.wav'],
      );
    });

    test('rejects paths with no extension', () {
      expect(filterAudioFilePaths(['/music/track']), isEmpty);
    });
  });

  group('payloadFromOsPath', () {
    test('uses basename as title and filesystem source', () {
      final payload = payloadFromOsPath('/home/geo/mix/song.wav');
      expect(payload.source, TrackDragSource.filesystem);
      expect(payload.trackId, isNull);
      expect(payload.path, '/home/geo/mix/song.wav');
      expect(payload.title, 'song.wav');
    });
  });

  group('payloadFromListTrack', () {
    test('library row uses load-library source', () {
      final payload = payloadFromListTrack(
        _track(id: 't1', path: '/lib/a.wav', title: 'Alpha'),
        inLibrary: true,
      );
      expect(payload.source, TrackDragSource.library);
      expect(payload.trackId, 't1');
      expect(payload.path, '/lib/a.wav');
      expect(payload.title, 'Alpha');
      expect(trackLoadKind(payload), TrackLoadKind.library);
    });

    test('drive file uses filesystem load when not in library', () {
      final payload = payloadFromListTrack(
        _track(id: '/tmp/x.wav', path: '/tmp/x.wav', displayName: 'x.wav'),
        inLibrary: false,
      );
      expect(payload.source, TrackDragSource.filesystem);
      expect(payload.trackId, isNull);
      expect(trackLoadKind(payload), TrackLoadKind.path);
    });

    test('missing title uses displayName', () {
      final payload = payloadFromListTrack(
        _track(id: 't1', path: '/lib/a.wav', displayName: 'a.wav'),
        inLibrary: true,
      );
      expect(payload.title, 'a.wav');
    });

    test('empty title and displayName uses file name', () {
      final payload = payloadFromListTrack(
        _track(id: 't1', path: '/lib/untitled.wav', title: '', displayName: ''),
        inLibrary: true,
      );
      expect(payload.title, 'untitled.wav');
    });
  });

}
