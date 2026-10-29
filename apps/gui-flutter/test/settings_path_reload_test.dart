import 'dart:async';

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/models/bus_enums.dart';
import 'package:gui_flutter/core/utils/track_drag.dart';
import 'package:gui_flutter/providers/mixer/deck_load.dart';
import 'package:gui_flutter/providers/mixer/engine_providers.dart';
import 'package:gui_flutter/providers/settings/settings_providers.dart';

import 'support/bus_events.dart';

class _PendingDeckLoadTransport implements DeckLoadTransport {
  final started = Completer<void>();
  final completed = Completer<void>();

  @override
  Future<void> loadLibraryTrack({
    required int deckId,
    required String trackId,
  }) {
    started.complete();
    return completed.future;
  }

  @override
  Future<void> loadPath({required int deckId, required String path}) {
    started.complete();
    return completed.future;
  }
}

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
    await restartDecksFromSettings(
      container.read,
      previousDeckCount: 2,
      restart: () async {},
      deckCount: 2,
      loadLibraryTrack: ({required deckId, required trackId}) async {
        loads.add('lib:$deckId:$trackId');
      },
      loadPath: ({required deckId, required path}) async {
        loads.add('path:$deckId:$path');
      },
    );

    expect(loads, ['path:0:/music/drive-track.wav', 'lib:1:lib-1']);
  });

  test('empty path-only decks are skipped', () async {
    final container = ProviderContainer();
    addTearDown(container.dispose);

    final loads = <String>[];
    await restartDecksFromSettings(
      container.read,
      previousDeckCount: 2,
      restart: () async {},
      deckCount: 2,
      loadLibraryTrack: ({required deckId, required trackId}) async {
        loads.add('lib:$deckId:$trackId');
      },
      loadPath: ({required deckId, required path}) async {
        loads.add('path:$deckId:$path');
      },
    );

    expect(loads, isEmpty);
  });
  test('four-deck profile reloads tracks on C and D', () async {
    final container = ProviderContainer();
    addTearDown(container.dispose);
    container.read(engineUiProvider.notifier)
      ..setDeckTrackPath(2, '/music/deck-c.wav')
      ..setDeckTrackId(2, null)
      ..setDeckTrackPath(3, '/music/deck-d.wav')
      ..setDeckTrackId(3, 'lib-d');

    final loads = <String>[];
    await restartDecksFromSettings(
      container.read,
      previousDeckCount: 4,
      restart: () async {},
      deckCount: 4,
      loadLibraryTrack: ({required deckId, required trackId}) async {
        loads.add('lib:$deckId:$trackId');
      },
      loadPath: ({required deckId, required path}) async {
        loads.add('path:$deckId:$path');
      },
    );
    expect(loads, ['path:2:/music/deck-c.wav', 'lib:3:lib-d']);
  });

  test('surviving decks reload identities captured before hydration', () async {
    final container = ProviderContainer();
    addTearDown(container.dispose);
    final ui = container.read(engineUiProvider.notifier);
    ui
      ..setDeckTrackPath(0, '/music/deck-a.wav')
      ..setDeckTrackPath(1, '/music/deck-b.flac')
      ..setDeckTrackId(1, 'lib-b')
      ..setDeckTrackPath(2, '/music/removed-c.wav')
      ..setDeckTrackId(3, 'removed-d');

    final loads = <String>[];
    await restartDecksFromSettings(
      container.read,
      previousDeckCount: 4,
      deckCount: 2,
      restart: () async {
        for (var deckId = 0; deckId < 4; deckId++) {
          ui.apply(deckUpdatedEvent(deck: deckId));
        }
      },
      loadLibraryTrack: ({required deckId, required trackId}) async {
        loads.add('lib:$deckId:$trackId');
      },
      loadPath: ({required deckId, required path}) async {
        loads.add('path:$deckId:$path');
        ui.setDeckTrackId(1, null);
      },
    );

    expect(loads, ['path:0:/music/deck-a.wav', 'lib:1:lib-b']);
    expect(container.read(engineUiProvider).trackPathFor(2), isNull);
    expect(container.read(engineUiProvider).trackIdFor(3), isNull);
  });

  test('pending surviving load cannot commit identity after restart', () async {
    final transport = _PendingDeckLoadTransport();
    final container = ProviderContainer(
      overrides: [
        deckLoadTransportProvider.overrideWith((ref) async => transport),
      ],
    );
    addTearDown(container.dispose);
    final pending = loadPayloadOntoDeck(
      container.read,
      0,
      const TrackDragPayload(
        source: TrackDragSource.library,
        path: '/music/pending-a.wav',
        trackId: 'pending-a',
        title: 'Pending A',
      ),
    );
    await transport.started.future;

    final loads = <String>[];
    await restartDecksFromSettings(
      container.read,
      previousDeckCount: 4,
      deckCount: 2,
      restart: () async {
        container
            .read(engineUiProvider.notifier)
            .apply(deckUpdatedEvent(deck: 0));
      },
      loadLibraryTrack: ({required deckId, required trackId}) async {
        loads.add(trackId);
      },
      loadPath: ({required deckId, required path}) async {
        loads.add(path);
      },
    );
    transport.completed.complete();
    await pending;

    expect(loads, isEmpty);
    expect(container.read(engineUiProvider).trackIdFor(0), isNull);
    expect(container.read(engineUiProvider).trackPathFor(0), isNull);
    expect(container.read(deckLoadingProvider(0)), isFalse);
  });

  test('removed state is cleared only when restart succeeds', () async {
    final container = ProviderContainer();
    addTearDown(container.dispose);
    final ui = container.read(engineUiProvider.notifier);
    ui
      ..setDeckTrackPath(2, '/music/deck-c.wav')
      ..setDeckTrackId(3, 'lib-d');
    final entered = Completer<void>();
    final restarted = Completer<void>();
    final cutover = restartDecksFromSettings(
      container.read,
      previousDeckCount: 4,
      deckCount: 2,
      restart: () {
        entered.complete();
        return restarted.future;
      },
      loadLibraryTrack: ({required deckId, required trackId}) async {},
      loadPath: ({required deckId, required path}) async {},
    );
    await entered.future;

    expect(
      container.read(engineUiProvider).trackPathFor(2),
      '/music/deck-c.wav',
    );
    expect(container.read(engineUiProvider).trackIdFor(3), 'lib-d');
    restarted.complete();
    await cutover;

    for (final deckId in [2, 3]) {
      ui.apply(
        deckUpdatedEvent(
          deck: deckId,
          update: deckUpdated(
            deck: deckId,
            trackPath: '/music/stale.wav',
            trackId: 'stale',
            durationMs: 5000,
          ),
        ),
      );
      expect(container.read(engineUiProvider).trackPathFor(deckId), isNull);
      expect(container.read(engineUiProvider).trackIdFor(deckId), isNull);
    }

    final loads = <String>[];
    await restartDecksFromSettings(
      container.read,
      previousDeckCount: 2,
      deckCount: 4,
      restart: () async {},
      loadLibraryTrack: ({required deckId, required trackId}) async {
        loads.add(trackId);
      },
      loadPath: ({required deckId, required path}) async {
        loads.add(path);
      },
    );
    expect(loads, isEmpty);
  });

  test('failed restart preserves the old profile and its events', () async {
    final container = ProviderContainer();
    addTearDown(container.dispose);
    final ui = container.read(engineUiProvider.notifier);
    ui.setDeckTrackId(3, 'lib-d');

    await expectLater(
      restartDecksFromSettings(
        container.read,
        previousDeckCount: 4,
        deckCount: 2,
        restart: () async => throw StateError('restart failed'),
        loadLibraryTrack: ({required deckId, required trackId}) async {},
        loadPath: ({required deckId, required path}) async {},
      ),
      throwsStateError,
    );

    expect(container.read(engineUiProvider).trackIdFor(3), 'lib-d');
    ui.apply(
      deckUpdatedEvent(
        deck: 3,
        update: deckUpdated(deck: 3, trackId: 'new-d', durationMs: 5000),
      ),
    );
    expect(container.read(engineUiProvider).trackIdFor(3), 'new-d');
  });

  test('expansion accepts fresh empty-deck routing during restart', () async {
    final container = ProviderContainer();
    addTearDown(container.dispose);
    final ui = container.read(engineUiProvider.notifier);
    ui.setActiveDeckCount(2);

    await restartDecksFromSettings(
      container.read,
      previousDeckCount: 2,
      deckCount: 4,
      restart: () async {
        ui.apply(
          deckUpdatedEvent(
            deck: 3,
            update: deckUpdated(deck: 3, crossfaderAssign: CrossfaderAssign.b),
          ),
        );
      },
      loadLibraryTrack: ({required deckId, required trackId}) async {},
      loadPath: ({required deckId, required path}) async {},
    );

    expect(
      container.read(engineUiProvider).channelFor(3).crossfaderAssign,
      CrossfaderAssign.b,
    );
  });
}
