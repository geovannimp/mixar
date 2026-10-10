import 'dart:async';

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/mixer/engine_providers.dart';
import 'package:gui_flutter/mixer/track_drag.dart';

TrackDragPayload _payload(String id) => TrackDragPayload(
  source: TrackDragSource.library,
  trackId: id,
  path: '/tmp/$id.wav',
  title: id,
);

class _FakeDeckLoadTransport implements DeckLoadTransport {
  new(this._gates);

  final Map<String, Completer<void>> _gates;
  final List<String> loads = [];

  @override
  Future<void> loadLibraryTrack({
    required int deckId,
    required String trackId,
  }) async {
    loads.add(trackId);
    final gate = _gates[trackId];
    if (gate != null) {
      await gate.future;
    }
  }

  @override
  Future<void> loadPath({required int deckId, required String path}) async {
    loads.add(path);
  }
}

ProviderContainer _container(_FakeDeckLoadTransport fake) {
  return ProviderContainer(
    overrides: [
      deckLoadTransportProvider.overrideWith((ref) async => fake),
    ],
  );
}

void main() {
  test('A-then-B commits chrome for the latest load', () async {
    final gates = <String, Completer<void>>{
      'A': Completer<void>()..complete(),
      'B': Completer<void>()..complete(),
    };
    final fake = _FakeDeckLoadTransport(gates);
    final container = _container(fake);
    addTearDown(container.dispose);

    await loadPayloadOntoDeck(container.read, 0, _payload('A'));
    await loadPayloadOntoDeck(container.read, 0, _payload('B'));

    final ui = container.read(engineUiProvider);
    expect(ui.trackIdFor(0), 'B');
    expect(ui.trackPathFor(0), '/tmp/B.wav');
    expect(fake.loads, ['A', 'B']);
  });

  test('B-supersedes-A ignores stale chrome commit from A', () async {
    final gateA = Completer<void>();
    final gates = <String, Completer<void>>{
      'A': gateA,
      'B': Completer<void>()..complete(),
    };
    final fake = _FakeDeckLoadTransport(gates);
    final container = _container(fake);
    addTearDown(container.dispose);

    final loadA = loadPayloadOntoDeck(container.read, 0, _payload('A'));
    // Let A reach its gated engine load before B starts.
    await Future<void>.delayed(Duration.zero);
    expect(fake.loads, ['A']);

    await loadPayloadOntoDeck(container.read, 0, _payload('B'));
    expect(container.read(engineUiProvider).trackIdFor(0), 'B');

    gateA.complete();
    await loadA;

    final ui = container.read(engineUiProvider);
    expect(ui.trackIdFor(0), 'B');
    expect(ui.trackPathFor(0), '/tmp/B.wav');
    expect(fake.loads, ['A', 'B']);
  });
}
