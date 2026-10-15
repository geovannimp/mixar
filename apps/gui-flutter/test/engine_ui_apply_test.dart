import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/mixer/engine_providers.dart';
import 'package:gui_flutter/mixer/jog_ticks.dart';
import 'package:gui_flutter/src/rust/api/engine.dart';

import 'support/fake_engine_transport.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  group('EngineUi.apply → deckPlayheads / slip', () {
    test(
      'position puts playhead and slip shadow without changing snapshot',
      () {
        final container = ProviderContainer();
        addTearDown(container.dispose);
        final before = container.read(engineUiProvider);

        container
            .read(engineUiProvider.notifier)
            .apply(
              const EngineEvt(
                kind: EngineEvtKind.position,
                deckId: 0,
                positionMs: 1234,
                slipShadowPositionMs: 100,
              ),
            );

        expect(container.read(deckPlayheadsProvider)[0], 1234);
        expect(container.read(deckSlipShadowsProvider)[0], 100);
        expect(container.read(deckPositionMsProvider(0)), 1234);
        expect(container.read(deckSlipShadowMsProvider(0)), 100);
        expect(identical(container.read(engineUiProvider), before), isTrue);
      },
    );

    test('position put is a no-op when ms is unchanged', () {
      final container = ProviderContainer();
      addTearDown(container.dispose);
      final ui = container.read(engineUiProvider.notifier);
      ui.apply(
        const EngineEvt(
          kind: EngineEvtKind.position,
          deckId: 0,
          positionMs: 500,
        ),
      );
      final map = container.read(deckPlayheadsProvider);
      ui.apply(
        const EngineEvt(
          kind: EngineEvtKind.position,
          deckId: 0,
          positionMs: 500,
        ),
      );
      expect(identical(container.read(deckPlayheadsProvider), map), isTrue);
    });

    test(
      'position without deckId or positionMs leaves playheads unchanged',
      () {
        final container = ProviderContainer();
        addTearDown(container.dispose);
        container
            .read(engineUiProvider.notifier)
            .apply(
              const EngineEvt(kind: EngineEvtKind.position, positionMs: 9),
            );
        container
            .read(engineUiProvider.notifier)
            .apply(const EngineEvt(kind: EngineEvtKind.position, deckId: 0));
        expect(container.read(deckPlayheadsProvider), isEmpty);
      },
    );

    test('updated puts playhead and slipEnabled on snapshot', () {
      final container = ProviderContainer();
      addTearDown(container.dispose);

      container
          .read(engineUiProvider.notifier)
          .apply(
            const EngineEvt(
              kind: EngineEvtKind.updated,
              deckId: 0,
              trackPath: '/a.wav',
              trackId: 't',
              durationMs: 8000,
              durationKnown: true,
              positionMs: 500,
              slipEnabled: true,
              slipShadowPositionMs: 400,
            ),
          );

      expect(container.read(deckPlayheadsProvider)[0], 500);
      expect(container.read(deckSlipShadowsProvider)[0], 400);
      expect(container.read(deckSlipEnabledProvider(0)), isTrue);
      expect(container.read(engineUiProvider).trackIdFor(0), 't');
    });

    test(
      'updated unload removes playhead and slip even when positionMs is 0',
      () {
        final container = ProviderContainer();
        addTearDown(container.dispose);
        final ui = container.read(engineUiProvider.notifier);
        ui.apply(
          const EngineEvt(
            kind: EngineEvtKind.position,
            deckId: 0,
            positionMs: 2200,
            slipShadowPositionMs: 1800,
          ),
        );
        expect(container.read(deckPlayheadsProvider), containsPair(0, 2200));
        expect(container.read(deckSlipShadowsProvider), containsPair(0, 1800));

        ui.apply(
          const EngineEvt(
            kind: EngineEvtKind.updated,
            deckId: 0,
            durationKnown: true,
            positionMs: 0,
          ),
        );

        expect(container.read(deckPlayheadsProvider).containsKey(0), isFalse);
        expect(container.read(deckSlipShadowsProvider).containsKey(0), isFalse);
        expect(container.read(deckPositionMsProvider(0)), 0);
        expect(container.read(engineUiProvider).trackIdFor(0), isNull);
      },
    );

    test('position evt with null slip shadow clears the cursor', () {
      final container = ProviderContainer();
      addTearDown(container.dispose);
      final ui = container.read(engineUiProvider.notifier);
      // Boundary: a null shadow before any set removes a non-existent key.
      ui.apply(
        const EngineEvt(
          kind: EngineEvtKind.position,
          deckId: 0,
          positionMs: 1000,
          slipShadowPositionMs: null,
        ),
      );
      expect(container.read(deckSlipShadowMsProvider(0)), isNull);
      expect(container.read(deckSlipShadowsProvider), isEmpty);
      ui.apply(
        const EngineEvt(
          kind: EngineEvtKind.position,
          deckId: 0,
          positionMs: 1234,
          slipShadowPositionMs: 100,
        ),
      );
      expect(container.read(deckSlipShadowMsProvider(0)), 100);
      ui.apply(
        const EngineEvt(
          kind: EngineEvtKind.position,
          deckId: 0,
          positionMs: 1300,
          slipShadowPositionMs: null,
        ),
      );
      expect(container.read(deckSlipShadowMsProvider(0)), isNull);
      expect(container.read(deckPlayheadsProvider)[0], 1300);
      // Round-trip: a later non-null shadow re-arms after the clear, so the
      // remove path cannot leave stale state in the provider map.
      ui.apply(
        const EngineEvt(
          kind: EngineEvtKind.position,
          deckId: 0,
          positionMs: 1400,
          slipShadowPositionMs: 1350,
        ),
      );
      expect(container.read(deckSlipShadowMsProvider(0)), 1350);
    });

    test('updated evt with null slip shadow clears the cursor', () {
      final container = ProviderContainer();
      addTearDown(container.dispose);
      final ui = container.read(engineUiProvider.notifier);
      // Boundary: a null shadow before any set removes a non-existent key.
      ui.apply(
        const EngineEvt(
          kind: EngineEvtKind.updated,
          deckId: 0,
          trackPath: '/a.wav',
          trackId: 't',
          durationMs: 8000,
          durationKnown: true,
          positionMs: 100,
          slipEnabled: true,
          slipShadowPositionMs: null,
        ),
      );
      expect(container.read(deckSlipShadowMsProvider(0)), isNull);
      expect(container.read(deckSlipShadowsProvider), isEmpty);
      ui.apply(
        const EngineEvt(
          kind: EngineEvtKind.updated,
          deckId: 0,
          trackPath: '/a.wav',
          trackId: 't',
          durationMs: 8000,
          durationKnown: true,
          positionMs: 500,
          slipEnabled: true,
          slipShadowPositionMs: 400,
        ),
      );
      expect(container.read(deckSlipShadowMsProvider(0)), 400);
      ui.apply(
        const EngineEvt(
          kind: EngineEvtKind.updated,
          deckId: 0,
          trackPath: '/a.wav',
          trackId: 't',
          durationMs: 8000,
          durationKnown: true,
          positionMs: 700,
          slipEnabled: true,
          slipShadowPositionMs: null,
        ),
      );
      expect(container.read(deckSlipShadowMsProvider(0)), isNull);
      expect(container.read(deckSlipEnabledProvider(0)), isTrue);
      // Round-trip: a later non-null shadow re-arms after the clear.
      ui.apply(
        const EngineEvt(
          kind: EngineEvtKind.updated,
          deckId: 0,
          trackPath: '/a.wav',
          trackId: 't',
          durationMs: 8000,
          durationKnown: true,
          positionMs: 900,
          slipEnabled: true,
          slipShadowPositionMs: 850,
        ),
      );
      expect(container.read(deckSlipShadowMsProvider(0)), 850);
    });
  });

  group('jogTurn paused + touching nudge', () {
    Future<WidgetRef> captureRef(
      WidgetTester tester,
      ProviderContainer container,
    ) async {
      WidgetRef? captured;
      await tester.pumpWidget(
        UncontrolledProviderScope(
          container: container,
          child: Consumer(
            builder: (context, ref, _) {
              captured = ref;
              return const SizedBox.shrink();
            },
          ),
        ),
      );
      return captured!;
    }

    testWidgets('nudges playhead from vinyl ticks when paused and touching', (
      tester,
    ) async {
      final engine = FakeEngineTransport();
      final container = ProviderContainer(
        overrides: [
          engineTransportProvider.overrideWith((ref) async => engine),
        ],
      );
      addTearDown(container.dispose);

      container
          .read(engineUiProvider.notifier)
          .apply(
            const EngineEvt(
              kind: EngineEvtKind.updated,
              deckId: 0,
              trackPath: '/a.wav',
              trackId: 't',
              durationMs: 60000,
              durationKnown: true,
              playing: false,
              positionMs: 1000,
            ),
          );
      container.read(engineUiProvider.notifier).setJogTouching(0, true);

      final ref = await captureRef(tester, container);
      await jogTurn(ref, 0, kJogIntervalsPerRev);
      await tester.pump();

      expect(
        container.read(deckPlayheadsProvider)[0],
        1000 + vinylTicksToDeltaMs(kJogIntervalsPerRev),
      );
      expect(engine.jogTurns, [(0, kJogIntervalsPerRev)]);
    });

    testWidgets('does not nudge playhead while playing', (tester) async {
      final engine = FakeEngineTransport();
      final container = ProviderContainer(
        overrides: [
          engineTransportProvider.overrideWith((ref) async => engine),
        ],
      );
      addTearDown(container.dispose);

      container
          .read(engineUiProvider.notifier)
          .apply(
            const EngineEvt(
              kind: EngineEvtKind.updated,
              deckId: 0,
              trackPath: '/a.wav',
              trackId: 't',
              durationMs: 60000,
              durationKnown: true,
              playing: true,
              positionMs: 1000,
            ),
          );
      container.read(engineUiProvider.notifier).setJogTouching(0, true);

      final ref = await captureRef(tester, container);
      await jogTurn(ref, 0, kJogIntervalsPerRev);
      await tester.pump();

      expect(container.read(deckPlayheadsProvider)[0], 1000);
      expect(engine.jogTurns, [(0, kJogIntervalsPerRev)]);
    });
  });
}
