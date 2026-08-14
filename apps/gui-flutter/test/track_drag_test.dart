import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/mixer/track_drag.dart';
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

  group('payloadFromLibraryTrack', () {
    test('library id uses load-library source', () {
      final payload = payloadFromLibraryTrack(
        _track(id: 't1', path: '/lib/a.wav', title: 'Alpha'),
      );
      expect(payload.source, TrackDragSource.library);
      expect(payload.trackId, 't1');
      expect(payload.path, '/lib/a.wav');
      expect(payload.title, 'Alpha');
      expect(trackLoadKind(payload), TrackLoadKind.library);
    });

    test('drive file with id==path uses filesystem load', () {
      final payload = payloadFromLibraryTrack(
        _track(id: '/tmp/x.wav', path: '/tmp/x.wav', displayName: 'x.wav'),
      );
      expect(payload.source, TrackDragSource.filesystem);
      expect(payload.trackId, isNull);
      expect(trackLoadKind(payload), TrackLoadKind.path);
    });

    test('missing title uses displayName', () {
      final payload = payloadFromLibraryTrack(
        _track(id: 't1', path: '/lib/a.wav', displayName: 'a.wav'),
      );
      expect(payload.title, 'a.wav');
    });

    test('empty title and displayName uses file name', () {
      final payload = payloadFromLibraryTrack(
        _track(id: 't1', path: '/lib/untitled.wav', title: '', displayName: ''),
      );
      expect(payload.title, 'untitled.wav');
    });
  });

  group('parseTrackDragLocalData', () {
    test('round-trips localData map', () {
      final original = payloadFromOsPath('/tmp/a.flac');
      expect(parseTrackDragLocalData(original.toLocalData()), original);
    });

    test('rejects non-track maps', () {
      expect(parseTrackDragLocalData({'type': 'other'}), isNull);
      expect(parseTrackDragLocalData('nope'), isNull);
    });

    test('empty title falls back to file name', () {
      expect(
        parseTrackDragLocalData({
          'type': 'track',
          'source': 'filesystem',
          'path': '/music/cut.mp3',
          'title': '',
        })?.title,
        'cut.mp3',
      );
    });
  });

  group('track drag plain text', () {
    test('round-trips JSON for GTK / OS clipboard formats', () {
      final original = TrackDragPayload(
        source: TrackDragSource.library,
        trackId: 't1',
        path: '/lib/a.wav',
        title: 'Alpha',
      );
      expect(
        parseTrackDragPlainText(encodeTrackDragPlainText(original)),
        original,
      );
    });

    test('rejects non-JSON and non-track text', () {
      expect(parseTrackDragPlainText('not json'), isNull);
      expect(parseTrackDragPlainText('{"type":"other"}'), isNull);
      expect(parseTrackDragPlainText(null), isNull);
    });

    test('rejects unsupported filesystem paths from OS plain text', () {
      expect(
        parseTrackDragPlainText(
          encodeTrackDragPlainText(
            const TrackDragPayload(
              source: TrackDragSource.filesystem,
              path: '/tmp/readme.txt',
              title: 'readme.txt',
            ),
          ),
        ),
        isNull,
      );
    });
  });

  group('preferredTrackDropOperation', () {
    test('prefers copy when the source allows it', () {
      expect(preferredTrackDropOperation({'move', 'copy'}), 'copy');
    });

    test('falls back to the first allowed op when copy is absent', () {
      expect(preferredTrackDropOperation({'move'}), 'move');
      expect(preferredTrackDropOperation(const {}), 'copy');
    });
  });

  group('pathFromDroppedUri', () {
    test('converts file uri to path', () {
      expect(
        pathFromDroppedUri(Uri.parse('file:///home/geo/a.wav')),
        '/home/geo/a.wav',
      );
    });
  });
}
