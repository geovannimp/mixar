import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/library/history_providers.dart';
import 'package:gui_flutter/src/rust/api/library.dart';

void main() {
  test('normalizeHistoryLocation strips file URI', () {
    expect(normalizeHistoryLocation('file:///music/a.flac'), '/music/a.flac');
    expect(normalizeHistoryLocation('/music/b.wav'), '/music/b.wav');
  });

  test('sessionPlayedKeysFromEntries prefers ids and paths', () {
    final keys = sessionPlayedKeysFromEntries([
      const HistoryEntryInfo(
        id: 'e1',
        trackId: 't1',
        location: 'file:///music/a.flac',
        deck: 0,
        startedAt: '2026-01-01T00:00:00Z',
      ),
      const HistoryEntryInfo(
        id: 'e2',
        trackId: null,
        location: '/music/drive-only.mp3',
        deck: 1,
        startedAt: '2026-01-01T00:01:00Z',
      ),
    ]);
    expect(keys.matches(trackId: 't1', path: '/other'), isTrue);
    expect(keys.matches(trackId: 'nope', path: '/music/a.flac'), isTrue);
    expect(
      keys.matches(trackId: 'nope', path: '/music/drive-only.mp3'),
      isTrue,
    );
    expect(keys.matches(trackId: 'nope', path: '/music/other.mp3'), isFalse);
  });

  test('empty keys never match', () {
    expect(SessionPlayedKeys.empty.matches(trackId: 't1', path: '/x'), isFalse);
  });
}
