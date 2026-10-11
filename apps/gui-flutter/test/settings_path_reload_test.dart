import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/mixer/engine_providers.dart';
import 'package:gui_flutter/settings/settings_providers.dart';

void main() {
  test('path-only decks reload via loadPath after settings restart', () async {
    final container = ProviderContainer();
    addTearDown(container.dispose);

    container.read(engineUiProvider.notifier)
      ..setDeckTrackPath(0, '/music/drive-track.wav')
      ..setDeckTrackId(0, null)
      ..setDeckTrackPath(1, '/music/library-track.flac')
      ..setDeckTrackId(1, 'lib-1');

    final loads = <String>[];
    await reloadDecksAfterSettingsRestart(
      container.read,
      loadLibraryTrack: ({required deckId, required trackId}) async {
        loads.add('lib:$deckId:$trackId');
      },
      loadPath: ({required deckId, required path}) async {
        loads.add('path:$deckId:$path');
      },
    );

    expect(loads, [
      'path:0:/music/drive-track.wav',
      'lib:1:lib-1',
    ]);
  });

  test('empty path-only decks are skipped', () async {
    final container = ProviderContainer();
    addTearDown(container.dispose);

    final loads = <String>[];
    await reloadDecksAfterSettingsRestart(
      container.read,
      loadLibraryTrack: ({required deckId, required trackId}) async {
        loads.add('lib:$deckId:$trackId');
      },
      loadPath: ({required deckId, required path}) async {
        loads.add('path:$deckId:$path');
      },
    );

    expect(loads, isEmpty);
  });
}
