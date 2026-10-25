import 'dart:typed_data';

import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/utils/jog_ticks.dart';
import 'package:gui_flutter/providers/mixer/engine_providers.dart';
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
    test('profile removal clears a deck and invalidates pending loads', () {
      final container = ProviderContainer();
      addTearDown(container.dispose);
      final ui = container.read(engineUiProvider.notifier);
      ui.apply(
        const EngineEvt(
          kind: EngineEvtKind.updated,
          deckId: 3,
          trackPath: '/d.wav',
          trackId: 'track-d',
          durationMs: 8000,
          durationKnown: true,
          positionMs: 1200,
        ),
      );
      ui.setDeckTrackPath(1, '/b.wav');
      final generation = container
          .read(deckLoadGenerationProvider.notifier)
          .begin(3);
      container.read(deckLoadInFlightProvider.notifier).set(3, true);

      ui.clearDeckState(3);

      final state = container.read(engineUiProvider);
      expect(state.trackPathFor(3), isNull);
      expect(state.trackIdFor(3), isNull);
      expect(state.durationMsFor(3), isNull);
      expect(state.trackPathFor(1), '/b.wav');
      expect(container.read(deckPlayheadsProvider), isNot(contains(3)));
      expect(container.read(deckLoadInFlightProvider), isNot(contains(3)));
      expect(
        container
            .read(deckLoadGenerationProvider.notifier)
            .isCurrent(3, generation),
        isFalse,
      );
    });

    test('inactive deck events cannot restore cleared profile state', () {
      final container = ProviderContainer();
      addTearDown(container.dispose);
      final ui = container.read(engineUiProvider.notifier);
      ui.setActiveDeckCount(2);
      final before = container.read(engineUiProvider);

      for (final deckId in [2, 3]) {
        ui
          ..apply(
            EngineEvt(
              kind: EngineEvtKind.updated,
              deckId: deckId,
              trackPath: '/stale.wav',
              trackId: 'stale',
              durationKnown: true,
              durationMs: 8000,
              positionMs: 1200,
            ),
          )
          ..apply(
            EngineEvt(
              kind: EngineEvtKind.position,
              deckId: deckId,
              positionMs: 1500,
              slipShadowPositionMs: 1000,
            ),
          )
          ..apply(
            EngineEvt(
              kind: EngineEvtKind.levels,
              deckId: deckId,
              peakL: 0.5,
              peakR: 0.5,
            ),
          );
      }

      expect(identical(container.read(engineUiProvider), before), isTrue);
      expect(container.read(deckPlayheadsProvider), isEmpty);
      expect(container.read(deckSlipShadowsProvider), isEmpty);
    });

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
    test('updated crossfader assignment patches only its deck', () {
      final container = ProviderContainer();
      addTearDown(container.dispose);

      container
          .read(engineUiProvider.notifier)
          .apply(
            const EngineEvt(
              kind: EngineEvtKind.updated,
              deckId: 3,
              crossfaderAssign: CrossfaderAssign.thru,
            ),
          );

      final ui = container.read(engineUiProvider);
      expect(ui.channelFor(3).crossfaderAssign, CrossfaderAssign.thru);
      expect(ui.channelFor(2).crossfaderAssign, CrossfaderAssign.a);
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

  group('EngineUi.apply → shared FX unit', () {
    test('updated evt stores the shared slots and this deck routing', () {
      final container = ProviderContainer();
      addTearDown(container.dispose);

      container
          .read(engineUiProvider.notifier)
          .apply(
            EngineEvt(
              kind: EngineEvtKind.updated,
              deckId: 0,
              fxRouted: true,
              fx: [
                FxSlotChrome(
                  kind: FxKind.echo,
                  enabled: true,
                  wet: 0.25,
                  params: Float32List.fromList([0.5, 0.5, 0.0]),
                ),
                FxSlotChrome(
                  kind: FxKind.reverb,
                  enabled: false,
                  wet: 0.5,
                  params: Float32List(3),
                ),
                FxSlotChrome(
                  kind: FxKind.off,
                  enabled: false,
                  wet: 0.5,
                  params: Float32List(3),
                ),
              ],
            ),
          );

      final snapshot = container.read(engineUiProvider);
      expect(snapshot.fxSlots.length, 3);
      expect(snapshot.fxSlots.first.kind, FxKind.echo);
      expect(snapshot.fxSlots.first.enabled, isTrue);
      expect(snapshot.channelFor(0).fxRouted, isTrue);
      // Other decks keep the default: routing is per deck.
      expect(snapshot.channelFor(1).fxRouted, isFalse);
    });

    test('an updated evt with an unchanged FX payload keeps the list', () {
      final container = ProviderContainer();
      addTearDown(container.dispose);
      final ui = container.read(engineUiProvider.notifier);
      EngineEvt echoEvt({required bool enabled}) => EngineEvt(
        kind: EngineEvtKind.updated,
        deckId: 0,
        fx: [
          FxSlotChrome(
            kind: FxKind.echo,
            enabled: enabled,
            wet: 0.25,
            params: Float32List.fromList([0.5, 0.5, 0.0]),
          ),
        ],
      );

      ui.apply(echoEvt(enabled: true));
      final before = container.read(engineUiProvider).fxSlots;
      expect(before, hasLength(1));

      // A fresh list carrying the same values must not notify FX listeners.
      ui.apply(echoEvt(enabled: true));
      expect(container.read(engineUiProvider).fxSlots, same(before));

      // A real change replaces it.
      ui.apply(echoEvt(enabled: false));
      final after = container.read(engineUiProvider).fxSlots;
      expect(after, isNot(same(before)));
      expect(after.single.enabled, isFalse);
    });

    test('an updated evt without FX fields keeps the last shared config', () {
      final container = ProviderContainer();
      addTearDown(container.dispose);
      final ui = container.read(engineUiProvider.notifier);
      ui.apply(
        EngineEvt(
          kind: EngineEvtKind.updated,
          deckId: 0,
          fx: [
            FxSlotChrome(
              kind: FxKind.echo,
              enabled: true,
              wet: 0.25,
              params: Float32List.fromList([1.0, 0.5, 0.0]),
            ),
          ],
        ),
      );
      final before = container.read(engineUiProvider).fxSlots;

      ui.apply(
        const EngineEvt(kind: EngineEvtKind.updated, deckId: 0, volume: 0.4),
      );

      expect(container.read(engineUiProvider).fxSlots, same(before));
      expect(container.read(engineUiProvider).channelFor(0).fxRouted, isFalse);
    });
  });
}
