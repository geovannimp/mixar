// End-to-end smoke of the unified bus: real Rust engine, real FRB stream.
//
// This is the one test that proves the whole path — engine publishes a typed
// `Event` on the process-wide bus, FRB hands it to Dart as a sealed class, and
// Dart reads the payload — without a window interaction. Widget tests cover the
// UI mapping; this covers the bridge.
import 'dart:async';

import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/models/bus_event_payload.dart';
import 'package:gui_flutter/src/rust/api/bus.dart';
import 'package:gui_flutter/src/rust/api/engine.dart';
import 'package:gui_flutter/src/rust/api/event_mirrors.dart';
import 'package:gui_flutter/src/rust/api/library.dart';
import 'package:gui_flutter/src/rust/frb_generated.dart';

/// Next event matching [predicate], or a failure after [timeout].
Future<T> nextEvent<T extends Object>(
  Stream<Event> events,
  T? Function(Event) pick, {
  bool Function(T)? where,
  Duration timeout = const Duration(seconds: 10),
}) {
  final completer = Completer<T>();
  late final StreamSubscription<Event> sub;
  sub = events.listen((event) {
    final value = pick(event);
    if (value == null || (where != null && !where(value))) {
      return;
    }
    if (!completer.isCompleted) {
      completer.complete(value);
    }
    unawaited(sub.cancel());
  });
  return completer.future.timeout(
    timeout,
    onTimeout: () {
      unawaited(sub.cancel());
      throw StateError('no matching bus event after $timeout');
    },
  );
}

void main() {
  setUpAll(() async {
    await RustLib.init();
  });

  test('engine events arrive on the one bus stream', () async {
    final bus = await BusTransport.open();
    final library = await LibraryTransport.openInMemory();
    final engine = await EngineTransport.start(
      bus: bus,
      libraryTransport: library,
    );
    final events = bus.subscribeEvents().asBroadcastStream();

    // Mixer state: one EngineSnapshot carrying the new crossfader.
    final snapshotFuture = nextEvent(
      events,
      (event) => event.engineSnapshot?.status,
      where: (status) => (status.crossfader - 0.3).abs() < 1e-4,
    );
    await engine.setCrossfader(position: 0.3);
    final snapshot = await snapshotFuture;
    expect(snapshot.crossfader, closeTo(0.3, 1e-4));
    expect(snapshot.decks, isNotEmpty);

    // Deck state: a full DeckUpdated, not a patched field bag.
    final volumeFuture = nextEvent(
      events,
      (event) => event.deckUpdated,
      where: (update) =>
          update.deck == 0 && (update.volume - 0.25).abs() < 1e-4,
    );
    await engine.setVolume(deckId: 0, volume: 0.25);
    final update = await volumeFuture;
    expect(update.deck, 0);
    expect(update.volume, closeTo(0.25, 1e-4));
    // The snapshot form is authoritative: authored fields are always present.
    expect(update.eq.low, isA<double>());
    expect(update.fx, isNotEmpty);

    // Failures ride the same stream as typed Errors with the deck attached.
    final errorFuture = nextEvent(
      events,
      (event) => event.busError,
      where: (error) => error.message.contains('No track loaded'),
    );
    await engine.play(deckId: 0);
    final error = await errorFuture;
    expect(error.deck, 0);
  });
}
