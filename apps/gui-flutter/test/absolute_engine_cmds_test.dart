import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/mixer/engine_providers.dart';
import 'package:gui_flutter/src/rust/api/engine.dart';

void main() {
  group('sendDeckPlaying', () {
    test('absolute play intent survives MIDI mid-flight flip', () async {
      final log = <String>[];
      // UI chrome showed paused; user pressed Play.
      var snapPlaying = false;
      final wantPlaying = !snapPlaying;

      // MIDI lands Play before the UI cmd awaits engine transport.
      snapPlaying = true;
      expect(snapPlaying, isTrue);

      await sendDeckPlaying(
        deckId: 0,
        playing: wantPlaying,
        play: ({required deckId}) async => log.add('play:$deckId'),
        pause: ({required deckId}) async => log.add('pause:$deckId'),
      );

      // Snapshot toggle after await would pause; absolute intent plays.
      expect(log, ['play:0']);
    });

    test('absolute pause intent stays pause if MIDI already paused', () async {
      final log = <String>[];
      var snapPlaying = true;
      final wantPlaying = !snapPlaying;

      snapPlaying = false;

      await sendDeckPlaying(
        deckId: 1,
        playing: wantPlaying,
        play: ({required deckId}) async => log.add('play:$deckId'),
        pause: ({required deckId}) async => log.add('pause:$deckId'),
      );

      expect(log, ['pause:1']);
    });
  });

  group('EngineUi optimistic absolute mirrors', () {
    test('setPlaying then MIDI evt reconciles without invert', () {
      final container = ProviderContainer();
      addTearDown(container.dispose);

      container.read(engineUiProvider.notifier).setPlaying(0, true);
      expect(container.read(engineUiProvider).isPlaying(0), isTrue);

      // Controller published absolute pause.
      container
          .read(engineUiProvider.notifier)
          .apply(
            const EngineEvt(
              kind: EngineEvtKind.updated,
              deckId: 0,
              playing: false,
            ),
          );
      expect(container.read(engineUiProvider).isPlaying(0), isFalse);

      // UI click captured wantPlaying=true from prior chrome; absolute set.
      container.read(engineUiProvider.notifier).setPlaying(0, true);
      expect(container.read(engineUiProvider).isPlaying(0), isTrue);
    });

    test('setQuantize / setSlipEnabled / setKeyLock are absolute', () {
      final container = ProviderContainer();
      addTearDown(container.dispose);
      final ui = container.read(engineUiProvider.notifier);

      // Defaults: quantize on, slip/keyLock off.
      expect(container.read(engineUiProvider).quantizeFor(0), isTrue);
      expect(container.read(engineUiProvider).slipEnabledFor(0), isFalse);
      expect(container.read(engineUiProvider).keyLockFor(0), isFalse);

      const wantQuantize = false;
      const wantSlip = true;
      const wantKeyLock = true;
      ui
        ..setQuantize(0, wantQuantize)
        ..setSlipEnabled(0, wantSlip)
        ..setKeyLock(0, wantKeyLock);
      expect(container.read(engineUiProvider).quantizeFor(0), isFalse);
      expect(container.read(engineUiProvider).slipEnabledFor(0), isTrue);
      expect(container.read(engineUiProvider).keyLockFor(0), isTrue);

      // MIDI races the opposite polarity before EngineEvt from our cmds.
      ui.apply(
        const EngineEvt(
          kind: EngineEvtKind.updated,
          deckId: 0,
          quantize: true,
          slipEnabled: false,
          keyLock: false,
        ),
      );
      expect(container.read(engineUiProvider).quantizeFor(0), isTrue);
      expect(container.read(engineUiProvider).slipEnabledFor(0), isFalse);
      expect(container.read(engineUiProvider).keyLockFor(0), isFalse);

      // Re-apply the captured absolute UI intents (not !snapshot).
      ui
        ..setQuantize(0, wantQuantize)
        ..setSlipEnabled(0, wantSlip)
        ..setKeyLock(0, wantKeyLock);
      expect(container.read(engineUiProvider).quantizeFor(0), isFalse);
      expect(container.read(engineUiProvider).slipEnabledFor(0), isTrue);
      expect(container.read(engineUiProvider).keyLockFor(0), isTrue);
    });
  });

  group('runOptimisticDeckBool', () {
    test('rolls back when engine is null', () async {
      var value = false;
      await runOptimisticDeckBool<Object>(
        previous: false,
        desired: true,
        write: (next) => value = next,
        readCurrent: () => value,
        resolveEngine: () async => null,
        send: (_) async {},
      );
      expect(value, isFalse);
    });

    test('rolls back and rethrows when send fails', () async {
      var value = false;
      final writes = <bool>[];
      await expectLater(
        runOptimisticDeckBool<Object>(
          previous: false,
          desired: true,
          write: (next) {
            value = next;
            writes.add(next);
          },
          readCurrent: () => value,
          resolveEngine: () async => Object(),
          send: (_) async => throw StateError('boom'),
        ),
        throwsStateError,
      );
      expect(writes, [true, false]);
      expect(value, isFalse);
    });

    test('does not clobber when MIDI moved mirror before failure', () async {
      var value = false;
      final writes = <bool>[];
      await expectLater(
        runOptimisticDeckBool<Object>(
          previous: false,
          desired: true,
          write: (next) {
            value = next;
            writes.add(next);
          },
          readCurrent: () => value,
          resolveEngine: () async => Object(),
          send: (_) async {
            // MIDI absolute pause landed; skip rollback write.
            value = false;
            throw StateError('boom');
          },
        ),
        throwsStateError,
      );
      expect(writes, [true]);
      expect(value, isFalse);
    });
  });
}
