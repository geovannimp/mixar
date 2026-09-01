import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/mixer/deck_loop_panel.dart';
import 'package:gui_flutter/mixer/engine_ui.dart';
import 'package:gui_flutter/src/rust/api/engine.dart';
import 'package:gui_flutter/src/rust/api/library.dart';

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
      SavedLoopInfo(slot: 2, inMs: 0, outMs: 8000),
      SavedLoopInfo(slot: 0, inMs: 1000, outMs: 3000),
      SavedLoopInfo(slot: 1, inMs: 1000, outMs: 3000),
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

  test('applyEngineEvt stores and clears active_loop when known', () {
    var snap = applyEngineEvt(
      EngineUiSnapshot.empty,
      const EngineEvt(
        kind: EngineEvtKind.updated,
        deckId: 0,
        activeLoop: ActiveLoopInfo(inMs: 10, outMs: 20, active: true),
        activeLoopKnown: true,
      ),
    );
    expect(snap.activeLoopFor(0)?.inMs, 10);

    snap = applyEngineEvt(
      snap,
      const EngineEvt(
        kind: EngineEvtKind.updated,
        deckId: 0,
        activeLoopKnown: true,
      ),
    );
    expect(snap.activeLoopFor(0), isNull);

    snap = applyEngineEvt(
      EngineUiSnapshot.empty.copyWith(
        activeLoops: {0: const ActiveLoopInfo(inMs: 1, outMs: 2, active: true)},
      ),
      const EngineEvt(kind: EngineEvtKind.updated, deckId: 0, playing: true),
    );
    expect(snap.activeLoopFor(0)?.inMs, 1);
  });
}
