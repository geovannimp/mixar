import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/mixer/engine_ui.dart';
import 'package:gui_flutter/src/rust/api/engine.dart';

void main() {
  group('applyEngineEvt', () {
    test('status sets running; updated sets deck title', () {
      var snap = EngineUiSnapshot.empty;
      snap = applyEngineEvt(
        snap,
        const EngineEvt(kind: EngineEvtKind.status, running: true),
      );
      expect(snap.running, isTrue);

      snap = applyEngineEvt(
        snap,
        const EngineEvt(
          kind: EngineEvtKind.updated,
          deckId: 0,
          track: 'Loaded Title',
        ),
      );
      expect(snap.titleFor(0), 'Loaded Title');
      expect(snap.titleFor(1), isNull);
    });

    test(
      'empty updated track keeps host title (engine snapshots omit library fields)',
      () {
        var snap = applyEngineEvt(
          EngineUiSnapshot.empty,
          const EngineEvt(kind: EngineEvtKind.updated, deckId: 1, track: 'X'),
        );
        snap = applyEngineEvt(
          snap,
          const EngineEvt(kind: EngineEvtKind.updated, deckId: 1),
        );
        expect(snap.titleFor(1), 'X');
      },
    );

    test('empty updated trackId keeps previous trackId', () {
      var snap = applyEngineEvt(
        EngineUiSnapshot.empty,
        const EngineEvt(
          kind: EngineEvtKind.updated,
          deckId: 1,
          trackId: 'abc',
          durationMs: 8000,
        ),
      );
      snap = applyEngineEvt(
        snap,
        const EngineEvt(kind: EngineEvtKind.updated, deckId: 1, playing: true),
      );
      expect(snap.trackIdFor(1), 'abc');
      expect(snap.durationMsFor(1), 8000);
      expect(snap.isPlaying(1), isTrue);
    });

    test('updated playing flag is stored per deck', () {
      var snap = applyEngineEvt(
        EngineUiSnapshot.empty,
        const EngineEvt(kind: EngineEvtKind.updated, deckId: 0, playing: true),
      );
      expect(snap.isPlaying(0), isTrue);
      expect(snap.isPlaying(1), isFalse);
    });

    test('position events do not change titles', () {
      final snap = applyEngineEvt(
        EngineUiSnapshot.empty,
        const EngineEvt(kind: EngineEvtKind.updated, deckId: 0, track: 'Keep'),
      );
      final after = applyEngineEvt(
        snap,
        const EngineEvt(
          kind: EngineEvtKind.position,
          deckId: 0,
          positionMs: 12,
        ),
      );
      expect(after.titleFor(0), 'Keep');
      expect(identical(after, snap), isTrue);
    });

    test('updated stores trackId duration speed without requiring a title', () {
      final snap = applyEngineEvt(
        EngineUiSnapshot.empty,
        const EngineEvt(
          kind: EngineEvtKind.updated,
          deckId: 1,
          trackId: 'abc',
          durationMs: 8000,
          speed: 0.62,
          tempoRange: 0.08,
          playing: false,
        ),
      );
      expect(snap.trackIdFor(1), 'abc');
      expect(snap.durationMsFor(1), 8000);
      expect(snap.speedFor(1), 0.62);
      expect(snap.tempoRangeFor(1), 0.08);
      expect(snap.isPlaying(1), isFalse);
    });

    test('updated mixer fields patch the channel', () {
      final snap = applyEngineEvt(
        EngineUiSnapshot.empty,
        const EngineEvt(
          kind: EngineEvtKind.updated,
          deckId: 0,
          volume: 0.25,
          eqHigh: 0.1,
          headphoneCue: true,
        ),
      );
      expect(snap.channelFor(0).volume, 0.25);
      expect(snap.channelFor(0).eqHigh, 0.1);
      expect(snap.channelFor(0).headphoneCue, isTrue);
      expect(snap.channelFor(0).eqLow, 0.5);
      expect(snap.channelFor(1).volume, 1.0);
    });

    test('status sets crossfader without clobbering volume', () {
      var snap = applyEngineEvt(
        EngineUiSnapshot.empty,
        const EngineEvt(kind: EngineEvtKind.updated, deckId: 0, volume: 0.3),
      );
      snap = applyEngineEvt(
        snap,
        const EngineEvt(
          kind: EngineEvtKind.status,
          running: true,
          crossfader: 0.2,
        ),
      );
      expect(snap.running, isTrue);
      expect(snap.crossfader, 0.2);
      expect(snap.channelFor(0).volume, 0.3);
    });

    test('levels patch peaks without changing volume', () {
      var snap = applyEngineEvt(
        EngineUiSnapshot.empty,
        const EngineEvt(kind: EngineEvtKind.updated, deckId: 0, volume: 0.4),
      );
      snap = applyEngineEvt(
        snap,
        const EngineEvt(
          kind: EngineEvtKind.levels,
          deckId: 0,
          peakL: 0.5,
          peakR: 0.6,
          peakHoldL: 0.8,
          peakHoldR: 0.9,
        ),
      );
      expect(snap.channelFor(0).volume, 0.4);
      expect(snap.levelsFor(0).peakL, 0.5);
      expect(snap.levelsFor(0).peakHoldR, 0.9);
    });

    test('updated padMode lands on the snapshot', () {
      final snap = applyEngineEvt(
        EngineUiSnapshot.empty,
        const EngineEvt(
          kind: EngineEvtKind.updated,
          deckId: 0,
          padMode: PadMode.loopRoll,
        ),
      );
      expect(snap.padModeFor(0), PadMode.loopRoll);
    });

    test('updated stores per-deck syncMode', () {
      var snap = applyEngineEvt(
        EngineUiSnapshot.empty,
        const EngineEvt(
          kind: EngineEvtKind.updated,
          deckId: 1,
          syncMode: SyncMode.tempo,
        ),
      );
      expect(snap.syncModeFor(1), SyncMode.tempo);
      expect(snap.syncModeFor(0), SyncMode.off);

      snap = applyEngineEvt(
        snap,
        const EngineEvt(kind: EngineEvtKind.updated, deckId: 1),
      );
      expect(snap.syncModeFor(1), SyncMode.tempo);
    });

    test('status sets masterDeck; default is deck 0', () {
      expect(EngineUiSnapshot.empty.masterDeck, 0);
      expect(EngineUiSnapshot.empty.isMaster(0), isTrue);
      expect(EngineUiSnapshot.empty.isMaster(1), isFalse);

      final snap = applyEngineEvt(
        EngineUiSnapshot.empty,
        const EngineEvt(kind: EngineEvtKind.status, masterDeck: 1),
      );
      expect(snap.masterDeck, 1);
      expect(snap.isMaster(1), isTrue);
      expect(snap.isMaster(0), isFalse);
    });
  });
}
