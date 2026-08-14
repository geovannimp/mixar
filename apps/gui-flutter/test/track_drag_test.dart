import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/mixer/track_drag.dart';
import 'package:gui_flutter/src/rust/api/engine.dart';
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
        _track(
          id: 't1',
          path: '/lib/a.wav',
          displayName: 'a.wav',
        ),
      );
      expect(payload.title, 'a.wav');
    });

    test('empty title and displayName uses file name', () {
      final payload = payloadFromLibraryTrack(
        _track(
          id: 't1',
          path: '/lib/untitled.wav',
          title: '',
          displayName: '',
        ),
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

  group('applyEngineEvt', () {
    test('status sets running; updated sets deck title', () {
      var snap = EngineUiSnapshot.empty;
      snap = applyEngineEvt(
        snap,
        const EngineEvt(kind: EngineEvtKind.status, running: true),
      );
      expect(snap.running, isTrue);

      snap = applyEngineEvt(
        snap,
        const EngineEvt(
          kind: EngineEvtKind.updated,
          deckId: 0,
          track: 'Loaded Title',
        ),
      );
      expect(snap.titleFor(0), 'Loaded Title');
      expect(snap.titleFor(1), isNull);
    });

    test(
      'empty updated track keeps host title (engine snapshots omit library fields)',
      () {
        var snap = applyEngineEvt(
          EngineUiSnapshot.empty,
          const EngineEvt(kind: EngineEvtKind.updated, deckId: 1, track: 'X'),
        );
        snap = applyEngineEvt(
          snap,
          const EngineEvt(kind: EngineEvtKind.updated, deckId: 1),
        );
        expect(snap.titleFor(1), 'X');
      },
    );

    test('updated playing flag is stored per deck', () {
      var snap = applyEngineEvt(
        EngineUiSnapshot.empty,
        const EngineEvt(kind: EngineEvtKind.updated, deckId: 0, playing: true),
      );
      expect(snap.isPlaying(0), isTrue);
      expect(snap.isPlaying(1), isFalse);
    });

    test('position events do not change titles', () {
      var snap = applyEngineEvt(
        EngineUiSnapshot.empty,
        const EngineEvt(kind: EngineEvtKind.updated, deckId: 0, track: 'Keep'),
      );
      snap = applyEngineEvt(
        snap,
        const EngineEvt(
          kind: EngineEvtKind.position,
          deckId: 0,
          positionMs: 12,
        ),
      );
      expect(snap.titleFor(0), 'Keep');
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
