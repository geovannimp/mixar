import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/providers/mixer/engine_providers.dart';
import 'package:gui_flutter/src/rust/api/engine.dart';

void main() {
  test('position put updates deckPositionMsProvider', () {
    final container = ProviderContainer();
    addTearDown(container.dispose);

    container
        .read(engineUiProvider.notifier)
        .apply(
          const EngineEvt(
            kind: EngineEvtKind.position,
            deckId: 0,
            positionMs: 1234,
          ),
        );

    expect(container.read(deckPositionMsProvider(0)), 1234);
    expect(container.read(deckPositionMsProvider(1)), 0);
  });

  test('duplicate position ms does not churn DeckPlayheads state', () {
    final container = ProviderContainer();
    addTearDown(container.dispose);
    final ui = container.read(engineUiProvider.notifier)
      ..apply(
        const EngineEvt(
          kind: EngineEvtKind.position,
          deckId: 0,
          positionMs: 500,
        ),
      );
    final first = container.read(deckPlayheadsProvider);

    ui.apply(
      const EngineEvt(kind: EngineEvtKind.position, deckId: 0, positionMs: 500),
    );
    final second = container.read(deckPlayheadsProvider);

    expect(identical(first, second), isTrue);
  });

  test('updated with duration seeds playhead from positionMs', () {
    final container = ProviderContainer();
    addTearDown(container.dispose);

    container
        .read(engineUiProvider.notifier)
        .apply(
          const EngineEvt(
            kind: EngineEvtKind.updated,
            deckId: 0,
            durationKnown: true,
            durationMs: 10000,
            positionMs: 2000,
          ),
        );

    expect(container.read(deckPositionMsProvider(0)), 2000);
  });

  test('updated unload removes playhead and slip shadow', () {
    final container = ProviderContainer();
    addTearDown(container.dispose);
    final ui = container.read(engineUiProvider.notifier)
      ..apply(
        const EngineEvt(
          kind: EngineEvtKind.position,
          deckId: 0,
          positionMs: 900,
          slipShadowPositionMs: 800,
        ),
      );
    expect(container.read(deckPositionMsProvider(0)), 900);
    expect(container.read(deckSlipShadowsProvider)[0], 800);

    ui.apply(
      const EngineEvt(
        kind: EngineEvtKind.updated,
        deckId: 0,
        durationKnown: true,
      ),
    );

    expect(container.read(deckPlayheadsProvider).containsKey(0), isFalse);
    expect(container.read(deckSlipShadowsProvider).containsKey(0), isFalse);
    expect(container.read(deckPositionMsProvider(0)), 0);
  });
}
