// End-to-end smoke of the unified bus: real Rust engine, real FRB stream.
//
// This is the one test that proves the whole path — engine publishes a typed
// `Event` on the process-wide bus, FRB hands it to Dart as a sealed class, and
// Dart reads the payload — without a window interaction. Widget tests cover the
// UI mapping; this covers the bridge and the command→event round trip for the
// paths a deck uses (load, transport, levels, jog, pads).
import 'dart:async';
import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/models/bus_event_payload.dart';
import 'package:gui_flutter/src/rust/api/bus.dart';
import 'package:gui_flutter/src/rust/api/engine.dart';
import 'package:gui_flutter/src/rust/api/event_mirrors.dart';
import 'package:gui_flutter/src/rust/api/library.dart';
import 'package:gui_flutter/src/rust/frb_generated.dart';

/// Next event matching [pick]/[where], or a failure after [timeout].
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

/// Repo fixture used by the Rust tests; loading it exercises the real decoder.
String shortToneFixture() =>
    File('../../samples/fixtures/short-tone.wav').absolute.path;

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
    final volumeUpdate = await volumeFuture;
    expect(volumeUpdate.deck, 0);
    expect(volumeUpdate.volume, closeTo(0.25, 1e-4));
    expect(volumeUpdate.eq.low, isA<double>());
    expect(volumeUpdate.fx, isNotEmpty);

    // Load a real file: the header/waveform need track id and duration, which
    // only a full snapshot carries.
    final loadedFuture = nextEvent(
      events,
      (event) => event.deckUpdated,
      where: (update) => update.deck == 0 && update.durationMs != null,
    );
    await engine.loadPath(deckId: 0, path: shortToneFixture());
    final loaded = await loadedFuture;
    expect(loaded.trackId, isNotNull);
    expect(loaded.durationMs, greaterThan(0));

    // Play: position and meter events follow on the same stream.
    final playingFuture = nextEvent(
      events,
      (event) => event.deckUpdated,
      where: (update) => update.deck == 0 && update.playing,
    );
    final levelsFuture = nextEvent(
      events,
      (event) => event.deckLevels,
      where: (levels) => levels.deck == 0,
    );
    await engine.play(deckId: 0);
    expect((await playingFuture).playing, isTrue);
    final levels = await levelsFuture;
    expect(levels.deck, 0);

    // Jog while playing: the playhead keeps moving through DeckPosition.
    final beforeJog = await nextEvent(
      events,
      (event) => event.deckPosition,
      where: (position) => position.deck == 0,
    );
    await engine.jogTurn(deckId: 0, delta: 240);
    final afterJog = await nextEvent(
      events,
      (event) => event.deckPosition,
      where: (position) =>
          position.deck == 0 && position.positionMs != beforeJog.positionMs,
    );
    expect(afterJog.positionMs, isNot(beforeJog.positionMs));

    // A pad press is a Signal: one press produces one event, never folded away.
    final padFuture = nextEvent(
      events,
      (event) => event.deckUpdated,
      where: (update) => update.deck == 0,
    );
    await engine.padPress(deckId: 0, slot: 0, shift: true);
    expect((await padFuture).deck, 0);

    // The deck-load path asks the library to refresh the track; that command
    // travels the same cmd bus as engine commands, and the engine must ignore it
    // instead of reporting "not an engine command" to the user.
    final refreshed = StringBuffer();
    final refreshSub = events.listen((event) {
      final error = event.busError;
      if (error != null && error.message.contains('not an engine command')) {
        refreshed.write(error.message);
      }
    });
    await library.refreshTrack(trackId: loaded.trackId!);
    await Future<void>.delayed(const Duration(milliseconds: 500));
    await refreshSub.cancel();
    expect(
      refreshed.toString(),
      isEmpty,
      reason: 'a library command must not surface as an engine error toast',
    );

    // Failures ride the same stream as typed Errors with the deck attached.
    final errorFuture = nextEvent(
      events,
      (event) => event.busError,
      where: (error) => error.message.contains('No track loaded'),
    );
    await engine.play(deckId: 1);
    expect((await errorFuture).deck, 1);
  });
}
