import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/models/bus_enums.dart';
import 'package:gui_flutter/core/models/engine_ui.dart';
import 'package:gui_flutter/core/models/pad_modes.dart';
import 'package:gui_flutter/src/rust/api/event_mirrors.dart' show DeckEq;

import 'support/bus_events.dart';

void main() {
  group('applyEngineEvent', () {
    test('snapshot sets running; update stores track path', () {
      var snap = EngineUiSnapshot.empty;
      snap = applyEngineEvent(snap, engineSnapshotEvent(running: true));
      expect(snap.running, isTrue);

      snap = applyEngineEvent(
        snap,
        deckUpdatedEvent(
          deck: 0,
          update: deckUpdated(
            deck: 0,
            trackPath: '/music/Loaded.opus',
            durationMs: 1000,
          ),
        ),
      );
      expect(snap.trackPathFor(0), '/music/Loaded.opus');
      expect(snap.trackPathFor(1), isNull);
    });

    test('update keeps previous track path when it carries none', () {
      var snap = applyEngineEvent(
        EngineUiSnapshot.empty,
        deckUpdatedEvent(
          deck: 1,
          update: deckUpdated(deck: 1, trackPath: '/x.wav', durationMs: 4000),
        ),
      );
      // A deck update is a full snapshot, so the deck stays loaded only while
      // it carries a duration; a null `track` leaves the path untouched.
      snap = applyEngineEvent(
        snap,
        deckUpdatedEvent(
          deck: 1,
          update: deckUpdated(deck: 1, durationMs: 4000),
        ),
      );
      expect(snap.trackPathFor(1), '/x.wav');
    });

    test('empty updated trackId keeps previous trackId', () {
      var snap = applyEngineEvent(
        EngineUiSnapshot.empty,
        deckUpdatedEvent(
          deck: 1,
          update: deckUpdated(deck: 1, trackId: 'abc', durationMs: 8000),
        ),
      );
      // Full snapshot: the duration is repeated, the identity fields are not,
      // so the previous identity survives.
      snap = applyEngineEvent(
        snap,
        deckUpdatedEvent(
          deck: 1,
          update: deckUpdated(deck: 1, playing: true, durationMs: 8000),
        ),
      );
      expect(snap.trackIdFor(1), 'abc');
      expect(snap.durationMsFor(1), 8000);
      expect(snap.isPlaying(1), isTrue);
    });

    test('updated playing flag is stored per deck', () {
      final snap = applyEngineEvent(
        EngineUiSnapshot.empty,
        deckUpdatedEvent(deck: 0, update: deckUpdated(deck: 0, playing: true)),
      );
      expect(snap.isPlaying(0), isTrue);
      expect(snap.isPlaying(1), isFalse);
    });

    test('position events do not change track paths', () {
      final snap = applyEngineEvent(
        EngineUiSnapshot.empty,
        deckUpdatedEvent(
          deck: 0,
          update: deckUpdated(
            deck: 0,
            trackPath: '/keep.wav',
            durationMs: 5000,
          ),
        ),
      );
      final after = applyEngineEvent(
        snap,
        deckPositionEvent(deck: 0, positionMs: 12),
      );
      expect(after.trackPathFor(0), '/keep.wav');
      expect(identical(after, snap), isTrue);
    });

    test('updated stores trackId duration speed without requiring a title', () {
      final snap = applyEngineEvent(
        EngineUiSnapshot.empty,
        deckUpdatedEvent(
          deck: 1,
          update: deckUpdated(
            deck: 1,
            trackId: 'abc',
            durationMs: 8000,
            speed: 0.62,
            tempoRange: 0.08,
          ),
        ),
      );
      expect(snap.trackIdFor(1), 'abc');
      expect(snap.durationMsFor(1), 8000);
      expect(snap.speedFor(1), 0.62);
      expect(snap.tempoRangeFor(1), 0.08);
      expect(snap.isPlaying(1), isFalse);
    });

    test('updated mixer fields patch the channel', () {
      final snap = applyEngineEvent(
        EngineUiSnapshot.empty,
        deckUpdatedEvent(
          deck: 0,
          update: deckUpdated(
            deck: 0,
            volume: 0.25,
            eq: const DeckEq(low: 0.5, mid: 0.5, high: 0.1),
            headphoneCue: true,
          ),
        ),
      );
      expect(snap.channelFor(0).volume, 0.25);
      expect(snap.channelFor(0).eqHigh, 0.1);
      expect(snap.channelFor(0).headphoneCue, isTrue);
      expect(snap.channelFor(0).eqLow, 0.5);
      expect(snap.channelFor(1).volume, 1.0);
    });

    test('snapshot sets cueMix and masterCue without clobbering volume', () {
      var snap = applyEngineEvent(
        EngineUiSnapshot.empty,
        deckUpdatedEvent(deck: 0, update: deckUpdated(deck: 0, volume: 0.3)),
      );
      snap = applyEngineEvent(
        snap,
        engineSnapshotEvent(
          masterVolume: 0.55,
          cueVolume: 0.25,
          cueMix: 0.4,
          masterCue: true,
        ),
      );
      expect(snap.running, isTrue);
      expect(snap.masterVolume, 0.55);
      expect(snap.cueVolume, 0.25);
      expect(snap.cueMix, 0.4);
      expect(snap.masterCue, isTrue);
      expect(snap.channelFor(0).volume, 0.3);
    });

    test('snapshot sets crossfader without clobbering volume', () {
      var snap = applyEngineEvent(
        EngineUiSnapshot.empty,
        deckUpdatedEvent(deck: 0, update: deckUpdated(deck: 0, volume: 0.3)),
      );
      snap = applyEngineEvent(snap, engineSnapshotEvent(crossfader: 0.2));
      expect(snap.running, isTrue);
      expect(snap.crossfader, 0.2);
      expect(snap.channelFor(0).volume, 0.3);
    });

    test('levels patch peaks without changing volume', () {
      var snap = applyEngineEvent(
        EngineUiSnapshot.empty,
        deckUpdatedEvent(deck: 0, update: deckUpdated(deck: 0, volume: 0.4)),
      );
      snap = applyEngineEvent(
        snap,
        deckLevelsEvent(
          deck: 0,
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

    test('levels without deckId patch masterLevels only', () {
      var snap = applyEngineEvent(
        EngineUiSnapshot.empty,
        deckLevelsEvent(deck: 0, peakL: 0.1, peakR: 0.2),
      );
      snap = applyEngineEvent(
        snap,
        deckLevelsEvent(
          peakL: 0.7,
          peakR: 0.8,
          peakHoldL: 0.9,
          peakHoldR: 0.95,
        ),
      );
      expect(snap.levelsFor(0).peakL, 0.1);
      expect(snap.masterLevels.peakL, 0.7);
      expect(snap.masterLevels.peakR, 0.8);
      expect(snap.masterLevels.peakHoldL, 0.9);
      expect(snap.masterLevels.peakHoldR, 0.95);
    });

    test('updated padMode lands on the snapshot', () {
      final snap = applyEngineEvent(
        EngineUiSnapshot.empty,
        deckUpdatedEvent(
          deck: 0,
          update: deckUpdated(deck: 0, padMode: PadMode.loopRoll),
        ),
      );
      expect(snap.padModeFor(0), PadMode.loopRoll);
    });

    test('updated stores per-deck syncMode', () {
      final snap = applyEngineEvent(
        EngineUiSnapshot.empty,
        deckUpdatedEvent(
          deck: 1,
          update: deckUpdated(deck: 1, syncMode: SyncMode.tempo),
        ),
      );
      expect(snap.syncModeFor(1), SyncMode.tempo);
      expect(snap.syncModeFor(0), SyncMode.off);
    });

    test('a later snapshot replaces the stored syncMode', () {
      // A deck update is a full snapshot, so the value it carries wins; there
      // is no patch-merge that would keep the earlier `tempo`.
      var snap = applyEngineEvent(
        EngineUiSnapshot.empty,
        deckUpdatedEvent(
          deck: 1,
          update: deckUpdated(
            deck: 1,
            durationMs: 1000,
            syncMode: SyncMode.tempo,
          ),
        ),
      );
      snap = applyEngineEvent(
        snap,
        deckUpdatedEvent(
          deck: 1,
          update: deckUpdated(
            deck: 1,
            durationMs: 1000,
            syncMode: SyncMode.beat,
          ),
        ),
      );
      expect(snap.syncModeFor(1), SyncMode.beat);
    });

    test('snapshot sets masterDeck; default is deck 0', () {
      expect(EngineUiSnapshot.empty.masterDeck, 0);
      expect(EngineUiSnapshot.empty.isMaster(0), isTrue);
      expect(EngineUiSnapshot.empty.isMaster(1), isFalse);

      final snap = applyEngineEvent(
        EngineUiSnapshot.empty,
        engineSnapshotEvent(masterDeck: 1),
      );
      expect(snap.masterDeck, 1);
      expect(snap.isMaster(1), isTrue);
      expect(snap.isMaster(0), isFalse);
    });

    test('updated stores quantize loudness autoGain and jogTouching', () {
      expect(EngineUiSnapshot.empty.quantizeFor(0), isTrue);
      final snap = applyEngineEvent(
        EngineUiSnapshot.empty,
        deckUpdatedEvent(
          deck: 0,
          update: deckUpdated(
            deck: 0,
            durationMs: 4000,
            quantize: false,
            jogTouching: true,
            loudnessLufs: -14.5,
            autoGainDb: -3.5,
          ),
        ),
      );
      expect(snap.quantizeFor(0), isFalse);
      expect(snap.jogTouchingFor(0), isTrue);
      expect(snap.loudnessLufsFor(0), -14.5);
      expect(snap.autoGainDbFor(0), -3.5);
    });

    test('updated stores slip on empty deck like quantize', () {
      final snap = applyEngineEvent(
        EngineUiSnapshot.empty,
        deckUpdatedEvent(
          deck: 0,
          update: deckUpdated(deck: 0, slipEnabled: true),
        ),
      );
      expect(snap.slipEnabledFor(0), isTrue);
    });

    test('duration_ms going null clears host identity and gain', () {
      var snap = applyEngineEvent(
        EngineUiSnapshot.empty,
        deckUpdatedEvent(
          deck: 0,
          update: deckUpdated(
            deck: 0,
            trackPath: '/keep.wav',
            trackId: 'abc',
            durationMs: 8000,
            loudnessLufs: -18,
            autoGainDb: 2,
            quantize: false,
          ),
        ),
      );
      snap = applyEngineEvent(
        snap,
        deckUpdatedEvent(
          deck: 0,
          update: deckUpdated(deck: 0, playing: false, quantize: false),
        ),
      );
      expect(snap.trackPathFor(0), isNull);
      expect(snap.trackIdFor(0), isNull);
      expect(snap.durationMsFor(0), isNull);
      expect(snap.loudnessLufsFor(0), isNull);
      expect(snap.autoGainDbFor(0), 0);
      expect(snap.quantizeFor(0), isFalse);
    });

    test('unload without prior duration still clears host identity', () {
      var snap = applyEngineEvent(
        EngineUiSnapshot.empty,
        deckUpdatedEvent(
          deck: 0,
          update: deckUpdated(
            deck: 0,
            trackPath: '/keep.wav',
            trackId: 'abc',
            durationMs: 8000,
          ),
        ),
      );
      snap = applyEngineEvent(
        snap,
        deckUpdatedEvent(deck: 0, update: deckUpdated(deck: 0)),
      );
      expect(snap.trackPathFor(0), isNull);
      expect(snap.trackIdFor(0), isNull);
      expect(snap.durationMsFor(0), isNull);
    });

    test('updated stores sampler bank id and slot chrome', () {
      final snap = applyEngineEvent(
        EngineUiSnapshot.empty,
        deckWithSamplerSlotEvent(activeSamplerBankId: 'bank-1'),
      );
      expect(snap.activeSamplerBankIdFor(0), 'bank-1');
      expect(snap.samplerSlotsFor(0).single.label, 'kick');
      expect(snap.samplerSlotsFor(0).single.path, '/samples/kick.wav');
      expect(snap.samplerSlotsFor(0).single.durationMs, 250);
    });

    test('unload clears sampler bank and slots with the deck', () {
      var snap = applyEngineEvent(
        EngineUiSnapshot.empty,
        deckWithSamplerSlotEvent(activeSamplerBankId: 'bank-1'),
      );
      expect(snap.activeSamplerBankIdFor(0), 'bank-1');
      // A null duration is the engine's unloaded snapshot: the bank and the
      // slots go with the track, exactly as the engine published them.
      snap = applyEngineEvent(
        snap,
        deckUpdatedEvent(deck: 0, update: deckUpdated(deck: 0)),
      );
      expect(snap.trackPathFor(0), isNull);
      expect(snap.trackIdFor(0), isNull);
      expect(snap.activeSamplerBankIdFor(0), isNull);
      expect(snap.samplerSlotsFor(0), isEmpty);
    });

    test('a mixer snapshot keeps the sampler chrome it cannot carry', () {
      // Pad chrome is host UI state: the engine snapshot has no slot to put it
      // in, so hydrating must keep what the mirror already shows. Wiping it here
      // would clear every pad label on any crossfader move.
      var snap = applyEngineEvent(
        EngineUiSnapshot.empty,
        deckWithSamplerSlotEvent(),
      );
      expect(snap.samplerSlotsFor(0).single.label, 'kick');

      snap = applyEngineEvent(
        snap,
        engineSnapshotEvent(
          crossfader: 0.2,
          decks: [deckSnapshot(deck: 0, trackId: 't1', durationMs: 1000)],
        ),
      );

      expect(snap.crossfader, 0.2);
      expect(snap.samplerSlotsFor(0).single.label, 'kick');
      expect(snap.samplerSlotsFor(0).single.path, '/samples/kick.wav');
    });

    test('an unloaded deck in a snapshot still clears its chrome', () {
      var snap = applyEngineEvent(
        EngineUiSnapshot.empty,
        deckWithSamplerSlotEvent(),
      );
      snap = applyEngineEvent(
        snap,
        engineSnapshotEvent(decks: [deckSnapshot(deck: 0)]),
      );
      expect(snap.samplerSlotsFor(0), isEmpty);
    });

    test('a host-stamped empty list clears a loaded deck chrome', () {
      // Bank load and clear_sampler publish a loaded deck whose slot list is
      // empty; that must clear, unlike a snapshot that simply cannot carry one.
      var snap = applyEngineEvent(
        EngineUiSnapshot.empty,
        deckWithSamplerSlotEvent(),
      );
      expect(snap.samplerSlotsFor(0), isNotEmpty);

      snap = applyEngineEvent(
        snap,
        deckUpdatedEvent(
          deck: 0,
          update: deckUpdated(deck: 0, trackId: 't1', durationMs: 1000),
        ),
      );
      expect(snap.samplerSlotsFor(0), isEmpty);
    });

    test('updated forwards stemsReady stemsLive mute and isolate', () {
      var snap = applyEngineEvent(
        EngineUiSnapshot.empty,
        deckUpdatedEvent(
          deck: 1,
          update: deckUpdated(
            deck: 1,
            durationMs: 1000,
            stemsReady: true,
            stemsLive: true,
            stemMute: const [true, false, false, false],
            stemIsolate: 2,
          ),
        ),
      );
      expect(snap.stemsReadyFor(1), isTrue);
      expect(snap.stemsLiveFor(1), isTrue);
      expect(snap.stemMuteFor(1), [true, false, false, false]);
      expect(snap.stemIsolateFor(1), 2);

      snap = applyEngineEvent(
        snap,
        deckUpdatedEvent(
          deck: 1,
          update: deckUpdated(
            deck: 1,
            durationMs: 1000,
            stemsReady: true,
            stemMute: const [false, false, false, false],
          ),
        ),
      );
      expect(snap.stemIsolateFor(1), isNull);

      snap = applyEngineEvent(
        snap,
        deckUpdatedEvent(deck: 1, update: deckUpdated(deck: 1)),
      );
      expect(snap.stemsReadyFor(1), isFalse);
      expect(snap.stemsLiveFor(1), isFalse);
      expect(snap.stemMuteFor(1), [false, false, false, false]);
    });

    test('updated evt carries key shift, per-mode pages and keyboard root', () {
      final snap = applyEngineEvent(
        EngineUiSnapshot.empty,
        deckUpdatedEvent(
          deck: 0,
          update: deckUpdated(
            deck: 0,
            durationMs: 1000,
            keyShift: 2,
            keyboardPage: 5,
            keyShiftPage: 3,
            keyboardRootHotCue: 3,
          ),
        ),
      );
      expect(snap.keyShiftFor(0), 2);
      expect(snap.keyboardPageFor(0), 5);
      expect(snap.keyShiftPageFor(0), 3);
      expect(snap.keyboardRootFor(0), 3);
      expect(snap.keyShiftFor(1), 0);
      expect(snap.keyboardPageFor(1), kDefaultPitchPage);
      expect(snap.keyShiftPageFor(1), kDefaultPitchPage);
      expect(snap.keyboardRootFor(1), 0);
    });
  });
}
