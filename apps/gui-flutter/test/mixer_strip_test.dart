import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/providers/mixer/engine_providers.dart';
import 'package:gui_flutter/providers/settings/settings_providers.dart';
import 'package:gui_flutter/src/rust/api/engine.dart';
import 'package:gui_flutter/widgets/mixer/fader_slider.dart';
import 'package:gui_flutter/widgets/mixer/mixer_strip.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:material_ui/material_ui.dart';

import 'support/fake_engine_transport.dart';
import 'support/mixar_material_app.dart';

class _RoutingTransport extends FakeEngineTransport {
  final assignments = <(int, CrossfaderAssign)>[];

  @override
  Future<void> setCrossfaderAssign({
    required int deckId,
    required CrossfaderAssign assignment,
  }) async {
    assignments.add((deckId, assignment));
  }
}

Future<({ProviderContainer container, _RoutingTransport engine})> _pumpStrip(
  WidgetTester tester, {
  int numDecks = 4,
  bool running = true,
}) async {
  final engine = _RoutingTransport();
  final container = ProviderContainer(
    overrides: [
      engineTransportProvider.overrideWith((ref) async => engine),
      appSettingsProvider.overrideWith(
        (ref) async => normalizeAppSettings(
          copyAppSettings(defaultAppSettings(), numDecks: numDecks),
        ),
      ),
    ],
  );
  addTearDown(container.dispose);
  addTearDown(engine.dispose);
  container
      .read(engineUiProvider.notifier)
      .apply(
        EngineEvt(
          kind: EngineEvtKind.status,
          running: running,
          crossfader: 0.35,
        ),
      );
  for (var id = 0; id < numDecks; id++) {
    _route(container, id, id.isEven ? CrossfaderAssign.a : CrossfaderAssign.b);
  }
  await tester.pumpWidget(
    UncontrolledProviderScope(
      container: container,
      child: mixarTestMaterialApp(
        theme: MixarThemeData.dark(),
        home: const Scaffold(
          body: Center(child: SizedBox(height: 500, child: MixerStrip())),
        ),
      ),
    ),
  );
  await tester.pumpAndSettle();
  return (container: container, engine: engine);
}

void _route(ProviderContainer container, int deckId, CrossfaderAssign side) {
  container
      .read(engineUiProvider.notifier)
      .apply(
        EngineEvt(
          kind: EngineEvtKind.updated,
          deckId: deckId,
          crossfaderAssign: side,
        ),
      );
}

Finder _endpoint(String side) => find
    .descendant(of: find.byType(MixerStrip), matching: find.text(side))
    .last;

bool _checked(WidgetTester tester, String deck) {
  final check = find
      .ancestor(
        of: find.text('Deck $deck'),
        matching: find.byWidgetPredicate(
          (widget) => widget is Semantics && widget.properties.checked != null,
        ),
      )
      .first;
  return tester.widget<Semantics>(check).properties.checked!;
}

void main() {
  testWidgets('side menus move decks exclusively and deselect to Thru', (
    tester,
  ) async {
    final (:container, :engine) = await _pumpStrip(tester);
    await tester.tap(_endpoint('A'));
    await tester.pumpAndSettle();
    expect(_checked(tester, 'A'), isTrue);
    expect(_checked(tester, 'B'), isFalse);
    expect(_checked(tester, 'C'), isTrue);
    expect(_checked(tester, 'D'), isFalse);

    await tester.tap(find.text('Deck C'));
    await tester.pumpAndSettle();
    expect(engine.assignments, [(2, CrossfaderAssign.thru)]);
    _route(container, 2, CrossfaderAssign.thru);
    await tester.pumpAndSettle();
    expect(_checked(tester, 'C'), isFalse);

    await tester.tap(find.text('Deck D'));
    await tester.pumpAndSettle();
    expect(engine.assignments.last, (3, CrossfaderAssign.a));
    _route(container, 3, CrossfaderAssign.a);
    await tester.pumpAndSettle();
    expect(_checked(tester, 'D'), isTrue);
    expect(_checked(tester, 'A'), isTrue);

    await tester.tapAt(const Offset(10, 10));
    await tester.pumpAndSettle();
    await tester.tap(_endpoint('B'));
    await tester.pumpAndSettle();
    expect(
      _checked(tester, 'D'),
      isFalse,
      reason: 'a deck cannot feed both sides',
    );
    expect(_checked(tester, 'B'), isTrue);
    expect(_checked(tester, 'C'), isFalse);
    await tester.tap(find.text('Deck C'));
    await tester.pumpAndSettle();
    expect(engine.assignments.last, (2, CrossfaderAssign.b));
    _route(container, 2, CrossfaderAssign.b);
    await tester.pumpAndSettle();
    expect(_checked(tester, 'C'), isTrue);
    final crossfader = tester.widget<FaderSlider>(
      find.byWidgetPredicate(
        (widget) =>
            widget is FaderSlider &&
            widget.orientation == FaderOrientation.horizontal,
      ),
    );
    expect(
      crossfader.value,
      closeTo(35, 1e-6),
      reason: 'routing must not move the fader',
    );
    await tester.pumpWidget(const SizedBox.shrink());
  });

  testWidgets('an open checklist follows external crossfader assignments', (
    tester,
  ) async {
    final (:container, :engine) = await _pumpStrip(tester);
    await tester.tap(_endpoint('A'));
    await tester.pumpAndSettle();
    expect(_checked(tester, 'C'), isTrue);
    _route(container, 2, CrossfaderAssign.b);
    await tester.pumpAndSettle();
    expect(_checked(tester, 'C'), isFalse);
    _route(container, 3, CrossfaderAssign.a);
    await tester.pumpAndSettle();
    expect(_checked(tester, 'D'), isTrue);
    expect(
      engine.assignments,
      isEmpty,
      reason: 'engine updates must not send commands',
    );
    await tester.pumpWidget(const SizedBox.shrink());
  });

  testWidgets('two-deck routing hides inactive decks', (tester) async {
    await _pumpStrip(tester, numDecks: 2);
    await tester.tap(_endpoint('B'));
    await tester.pumpAndSettle();
    expect(find.text('Deck A'), findsOneWidget);
    expect(find.text('Deck B'), findsOneWidget);
    expect(find.text('Deck C'), findsNothing);
    expect(find.text('Deck D'), findsNothing);
    await tester.pumpWidget(const SizedBox.shrink());
  });

  testWidgets('a stopped engine cannot open routing menus', (tester) async {
    final (:container, :engine) = await _pumpStrip(tester, running: false);
    await tester.tap(_endpoint('A'));
    await tester.pumpAndSettle();
    expect(find.text('Deck A'), findsNothing);
    await tester.tap(_endpoint('B'));
    await tester.pumpAndSettle();
    expect(find.text('Deck B'), findsNothing);
    expect(engine.assignments, isEmpty);
    expect(container.read(crossfaderProvider), 0.35);
    await tester.pumpWidget(const SizedBox.shrink());
  });
}
