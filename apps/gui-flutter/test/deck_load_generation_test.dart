import 'dart:async';

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/mixer/deck_load.dart';
import 'package:gui_flutter/mixer/engine_providers.dart';
import 'package:gui_flutter/mixer/track_drag.dart';
import 'package:gui_flutter/shell/app_notice.dart';

TrackDragPayload _payload(String id) => TrackDragPayload(
  source: TrackDragSource.library,
  trackId: id,
  path: '/tmp/$id.wav',
  title: id,
);

class _FakeDeckLoadTransport implements DeckLoadTransport {
  final List<String> loads = [];

  @override
  Future<void> loadLibraryTrack({
    required int deckId,
    required String trackId,
  }) async {
    loads.add(trackId);
  }

  @override
  Future<void> loadPath({required int deckId, required String path}) async {
    loads.add(path);
  }
}

class _ThrowingDeckLoadTransport implements DeckLoadTransport {
  @override
  Future<void> loadLibraryTrack({
    required int deckId,
    required String trackId,
  }) async {
    throw Exception('load failed');
  }

  @override
  Future<void> loadPath({required int deckId, required String path}) async {
    throw Exception('load failed');
  }
}

void main() {
  test('A-then-B commits chrome for the latest load', () async {
    final fake = _FakeDeckLoadTransport();
    final container = ProviderContainer(
      overrides: [deckLoadTransportProvider.overrideWith((ref) async => fake)],
    );
    addTearDown(container.dispose);

    await loadPayloadOntoDeck(container.read, 0, _payload('A'));
    await loadPayloadOntoDeck(container.read, 0, _payload('B'));

    final ui = container.read(engineUiProvider);
    expect(ui.trackIdFor(0), 'B');
    expect(ui.trackPathFor(0), '/tmp/B.wav');
    expect(fake.loads, ['A', 'B']);
  });

  test('B-supersedes-A skips transport for superseded A', () async {
    final gate = Completer<void>();
    final fake = _FakeDeckLoadTransport();
    final container = ProviderContainer(
      overrides: [
        deckLoadTransportProvider.overrideWith((ref) async {
          await gate.future;
          return fake;
        }),
      ],
    );
    addTearDown(container.dispose);

    final loadA = loadPayloadOntoDeck(container.read, 0, _payload('A'));
    // A is blocked on transport resolution before any engine call.
    await Future<void>.delayed(Duration.zero);
    expect(fake.loads, isEmpty);

    final loadB = loadPayloadOntoDeck(container.read, 0, _payload('B'));
    await Future<void>.delayed(Duration.zero);

    gate.complete();
    await Future.wait([loadA, loadB]);

    final ui = container.read(engineUiProvider);
    expect(ui.trackIdFor(0), 'B');
    expect(ui.trackPathFor(0), '/tmp/B.wav');
    expect(fake.loads, ['B']);
  });

  test('failed loadLibraryTrack sets appNotice error', () async {
    final container = ProviderContainer(
      overrides: [
        deckLoadTransportProvider.overrideWith(
          (ref) async => _ThrowingDeckLoadTransport(),
        ),
      ],
    );
    addTearDown(container.dispose);

    await loadPayloadOntoDeck(container.read, 0, _payload('bad'));

    final notice = container.read(appNoticeProvider);
    expect(notice, isNotNull);
    expect(notice!.kind, AppNoticeKind.error);
    expect(notice.message, contains('load failed'));
  });
}
