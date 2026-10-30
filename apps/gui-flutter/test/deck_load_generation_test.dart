import 'dart:async';

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/utils/track_drag.dart';
import 'package:gui_flutter/providers/library/drawer_providers.dart';
import 'package:gui_flutter/providers/mixer/deck_load.dart';
import 'package:gui_flutter/providers/mixer/engine_providers.dart';
import 'package:gui_flutter/providers/shell/app_notice.dart';

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

class _GatedThrowDeckLoadTransport implements DeckLoadTransport {
  _GatedThrowDeckLoadTransport(this._gate);

  final Completer<void> _gate;

  @override
  Future<void> loadLibraryTrack({
    required int deckId,
    required String trackId,
  }) async {
    await _gate.future;
    throw Exception('stale load failed');
  }

  @override
  Future<void> loadPath({required int deckId, required String path}) async {
    await _gate.future;
    throw Exception('stale load failed');
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

  test('superseded failed load does not set appNotice', () async {
    final gate = Completer<void>();
    final fake = _GatedThrowDeckLoadTransport(gate);
    final container = ProviderContainer(
      overrides: [deckLoadTransportProvider.overrideWith((ref) async => fake)],
    );
    addTearDown(container.dispose);

    final stale = loadPayloadOntoDeck(container.read, 0, _payload('stale'));
    await Future<void>.delayed(Duration.zero);
    // Begin a newer generation so the stale attempt is superseded.
    container.read(deckLoadGenerationProvider.notifier).begin(0);
    gate.complete();
    await stale;

    expect(container.read(appNoticeProvider), isNull);
  });

  test('load closes the open library drawer', () async {
    final fake = _FakeDeckLoadTransport();
    final container = ProviderContainer(
      overrides: [deckLoadTransportProvider.overrideWith((ref) async => fake)],
    );
    addTearDown(container.dispose);
    container.read(libraryDrawerOpenProvider.notifier).set(true);

    // The controller path reaches the same funnel as the UI ones.
    await loadPayloadOntoDeck(container.read, 0, _payload('A'));

    expect(container.read(libraryDrawerOpenProvider), isFalse);
  });

  test('load leaves the drawer up mid-drag to keep the drag source', () async {
    final fake = _FakeDeckLoadTransport();
    final container = ProviderContainer(
      overrides: [deckLoadTransportProvider.overrideWith((ref) async => fake)],
    );
    addTearDown(container.dispose);
    container.read(libraryDrawerOpenProvider.notifier).set(true);
    container.read(libraryTrackDragActiveProvider.notifier).begin();

    await loadPayloadOntoDeck(container.read, 0, _payload('A'));

    expect(container.read(libraryDrawerOpenProvider), isTrue);
  });
}
