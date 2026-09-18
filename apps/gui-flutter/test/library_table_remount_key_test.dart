import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/library/track_table_pane.dart';
import 'package:gui_flutter/mixer/key_format.dart';

void main() {
  test('library table remount key ignores play/harmonic state', () {
    final a = libraryTableRemountKey(
      sourceId: 'collection-1',
      tableColumns: const ['title', 'key'],
      keyColorMode: KeyColorMode.harmonic,
      keyDisplayMode: KeyDisplayMode.camelot,
    );
    final b = libraryTableRemountKey(
      sourceId: 'collection-1',
      tableColumns: const ['title', 'key'],
      keyColorMode: KeyColorMode.harmonic,
      keyDisplayMode: KeyDisplayMode.camelot,
    );
    expect(a, equals(b));

    // Column / mode changes must remount; play/pause must not (those are not
    // inputs here — regression guard against putting them back on the ValueKey).
    expect(
      libraryTableRemountKey(
        sourceId: 'collection-1',
        tableColumns: const ['title'],
        keyColorMode: KeyColorMode.harmonic,
        keyDisplayMode: KeyDisplayMode.camelot,
      ),
      isNot(equals(a)),
    );
  });
}
