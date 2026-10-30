import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/models/engine_ui.dart';
import 'package:gui_flutter/src/rust/api/event_mirrors.dart'
    show LoopRegion, SavedLoop;
import 'package:gui_flutter/widgets/mixer/deck_loop_panel.dart';

import 'support/bus_events.dart';

void main() {
  test('kAutoLoopBeats matches Tauri list', () {
    expect(kAutoLoopBeats, [1, 2, 4, 8, 16, 32]);
  });

  test('autoLoopBeatIndex falls back to 4', () {
    expect(autoLoopBeatIndex(4), 2);
    expect(autoLoopBeatIndex(99), 2);
  });

  test('stepAutoLoopBeats clamps at ends', () {
    expect(stepAutoLoopBeats(1, -1), 1);
    expect(stepAutoLoopBeats(1, 1), 2);
    expect(stepAutoLoopBeats(32, 1), 32);
    expect(stepAutoLoopBeats(32, -1), 16);
  });

  test('savedLoopAtPosition prefers tightest then lowest slot', () {
    const loops = [
      SavedLoop(slot: 2, inMs: 0, outMs: 8000),
      SavedLoop(slot: 0, inMs: 1000, outMs: 3000),
      SavedLoop(slot: 1, inMs: 1000, outMs: 3000),
    ];
    final hit = savedLoopAtPosition(loops, 2000);
    expect(hit?.slot, 0);
    expect(savedLoopAtPosition(loops, 9000), isNull);
  });

  test('beatsFromLoopMs snaps to nearest listed length', () {
    // 4 beats at 120 BPM = 2000 ms
    expect(beatsFromLoopMs(inMs: 0, outMs: 2000, bpm: 120), 4);
    expect(beatsFromLoopMs(inMs: 0, outMs: 1000, bpm: 120), 2);
    expect(beatsFromLoopMs(inMs: 0, outMs: 2000, bpm: null), 4);
  });

  test('applyEngineEvent stores and clears active_loop', () {
    var snap = applyEngineEvent(
      EngineUiSnapshot.empty,
      deckUpdatedEvent(
        deck: 0,
        update: deckUpdated(
          deck: 0,
          durationMs: 1000,
          activeLoop: const LoopRegion(inMs: 10, outMs: 20, active: true),
        ),
      ),
    );
    expect(snap.activeLoopFor(0)?.inMs, 10);

    snap = applyEngineEvent(
      snap,
      deckUpdatedEvent(deck: 0, update: deckUpdated(deck: 0, durationMs: 1000)),
    );
    expect(snap.activeLoopFor(0), isNull);

    // A loop that is carried but not active is not an active loop.
    snap = applyEngineEvent(
      snap,
      deckUpdatedEvent(
        deck: 0,
        update: deckUpdated(
          deck: 0,
          durationMs: 1000,
          activeLoop: const LoopRegion(inMs: 1, outMs: 2, active: false),
        ),
      ),
    );
    expect(snap.activeLoopFor(0), isNull);
  });

  test('applyEngineEvent stores and clears pending_loop_in', () {
    var snap = applyEngineEvent(
      EngineUiSnapshot.empty,
      deckUpdatedEvent(
        deck: 0,
        update: deckUpdated(deck: 0, durationMs: 1000, pendingLoopInMs: 1500),
      ),
    );
    expect(snap.pendingLoopInMsFor(0), 1500);

    snap = applyEngineEvent(
      snap,
      deckUpdatedEvent(deck: 0, update: deckUpdated(deck: 0, durationMs: 1000)),
    );
    expect(snap.pendingLoopInMsFor(0), isNull);
  });
}
