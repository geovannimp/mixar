import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/models/engine_ui.dart';
import 'package:gui_flutter/core/theme/deck_accent.dart';
import 'package:gui_flutter/core/utils/desktop.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/providers/mixer/engine_providers.dart';
import 'package:gui_flutter/providers/settings/settings_providers.dart';
import 'package:gui_flutter/providers/shell/controller_providers.dart';
import 'package:gui_flutter/widgets/mixer/deck_key_lock_button.dart';
import 'package:gui_flutter/widgets/mixer/deck_panel.dart';
import 'package:gui_flutter/widgets/mixer/deck_performance_panel.dart';
import 'package:gui_flutter/widgets/mixer/deck_tempo_panel.dart';
import 'package:gui_flutter/widgets/mixer/deck_track_info.dart';
import 'package:gui_flutter/widgets/shell/m_card.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';
import 'package:material_ui/material_ui.dart' show Scaffold;

import 'support/fake_engine_transport.dart';
import 'support/mixar_material_app.dart';

class _RunningEngineUi extends EngineUi {
  @override
  EngineUiSnapshot build() =>
      const EngineUiSnapshot(running: true, trackPaths: {});
}

void main() {
  Future<void> pumpDeck(
    WidgetTester tester, {
    Size size = const Size(370, 560),
    FakeEngineTransport? transport,
  }) async {
    debugOverrideDesktopWindow = false;
    addTearDown(() => debugOverrideDesktopWindow = null);
    tester.view.physicalSize = size;
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);

    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          engineUiProvider.overrideWith(_RunningEngineUi.new),
          engineTransportProvider.overrideWith(
            (ref) async => transport ?? FakeEngineTransport(),
          ),
          appSettingsProvider.overrideWith((ref) async => defaultAppSettings()),
          audioDevicesProvider.overrideWith((ref) async => const []),
          samplerBanksProvider.overrideWith((ref) async => const []),
          controllerTransportProvider.overrideWith((ref) async => null),
        ],
        child: mixarTestMaterialApp(
          theme: MixarThemeData.dark(),
          home: const Scaffold(
            body: DeckPanel(deckId: 0, label: 'DECK A', accent: DeckAccent.a),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();
  }

  testWidgets('deck header drops the deck name and gain details control', (
    tester,
  ) async {
    await pumpDeck(tester);

    expect(find.text('DECK A'), findsNothing);
    expect(find.byIcon(LucideIcons.info), findsNothing);
  });

  testWidgets('deck body runs from the column edge to the tempo rail', (
    tester,
  ) async {
    await pumpDeck(tester);

    final info = tester.getRect(find.byType(DeckTrackInfo));
    final rail = tester.getRect(find.byType(DeckTempoPanel));

    expect(info.left, 0, reason: 'no gutter against the window edge');
    expect(
      info.right,
      rail.left,
      reason: 'the details meet the tempo rail, no gap between them',
    );
  });

  testWidgets('toggle cluster keeps the title floor and scrolls into reach', (
    tester,
  ) async {
    await pumpDeck(tester);

    final title = tester.getRect(
      find.byKey(const ValueKey('deck-title-artist')),
    );
    expect(title.width, 56, reason: 'the cluster cap keeps the title floor');

    final crown = find.descendant(
      of: find.byType(DeckTrackInfo),
      matching: find.byIcon(LucideIcons.crown),
    );
    final scroll = find.descendant(
      of: find.byType(DeckTrackInfo),
      matching: find.byType(SingleChildScrollView),
    );
    final viewport = tester.getRect(scroll);

    expect(
      tester.getRect(crown).left,
      lessThan(viewport.left),
      reason: 'the narrow deck clips the leading toggle first',
    );

    await tester.drag(scroll, const Offset(200, 0));
    await tester.pumpAndSettle();

    expect(
      tester.getRect(crown).left,
      greaterThanOrEqualTo(viewport.left - 0.5),
      reason: 'the clipped toggle can be scrolled into reach',
    );
  });

  testWidgets('performance panel runs full-bleed with the track info', (
    tester,
  ) async {
    await pumpDeck(tester);

    final info = tester.getRect(find.byType(DeckTrackInfo));
    final panel = tester.getRect(find.byType(DeckPerformancePanel));

    expect(info.left, 0, reason: 'the track info stays full-bleed');
    expect(panel.left, info.left, reason: 'no side margin on the panel');
    expect(panel.right, info.right);
  });

  testWidgets('deck content starts at the pane top', (tester) async {
    await pumpDeck(tester);

    final info = tester.getTopLeft(find.byType(DeckTrackInfo));
    final rail = tester.getTopLeft(find.byType(DeckTempoPanel));

    expect(rail.dy, 0, reason: 'the rail spans the deck height');
    expect(info.dy, 0, reason: 'no 1 px margin above the deck content');
  });

  testWidgets('track info card keeps only its bottom hairline', (tester) async {
    await pumpDeck(tester);

    final theme = MixarThemeData.dark();
    final decorated = tester.widget<DecoratedBox>(
      find
          .descendant(
            of: find.byType(MCard),
            matching: find.byType(DecoratedBox),
          )
          .first,
    );
    final border = (decorated.decoration as BoxDecoration).border! as Border;

    expect(border.left, BorderSide.none);
    expect(border.right, BorderSide.none);
    expect(border.top, BorderSide.none);
    expect(border.bottom.width, theme.style.borderWidth);
  });

  testWidgets('deck toggles move into the track info card', (tester) async {
    await pumpDeck(tester);

    final info = find.byType(DeckTrackInfo);

    expect(
      find.descendant(of: info, matching: find.text('ST')),
      findsOneWidget,
    );
    expect(
      find.descendant(of: info, matching: find.byIcon(LucideIcons.crown)),
      findsOneWidget,
    );
    expect(
      find.descendant(of: info, matching: find.text('FX')),
      findsOneWidget,
    );
    expect(
      find.descendant(of: info, matching: find.byType(DeckKeyLockButton)),
      findsOneWidget,
      reason: 'the key joins the toggle cluster',
    );
    expect(
      find.text('ST'),
      findsOneWidget,
      reason: 'no second toggle cluster remains in a deck header',
    );
  });

  testWidgets('a narrow deck keeps the toggle cluster reachable', (
    tester,
  ) async {
    final transport = FakeEngineTransport();
    await pumpDeck(tester, size: const Size(205, 300), transport: transport);

    final viewport = find.descendant(
      of: find.byType(DeckTrackInfo),
      matching: find.byType(SingleChildScrollView),
    );
    expect(
      tester.getRect(viewport).width,
      greaterThan(0),
      reason: 'a zero-width scroller would strand every moved control',
    );
    expect(
      tester.getRect(find.byKey(const ValueKey('deck-title-artist'))).width,
      0,
      reason: 'the cluster takes the row when both cannot fit',
    );

    // Scroll the cluster to the FX chip and operate it (ST needs audible
    // stems; FX is live whenever the engine runs).
    await tester.ensureVisible(find.text('FX'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('FX'));
    await tester.pumpAndSettle();

    expect(transport.deckFxRouted, [(0, true)]);
  });

  testWidgets('the minimum desktop width keeps the title floor', (
    tester,
  ) async {
    await pumpDeck(tester, size: const Size(300, 560));

    expect(tester.takeException(), isNull);
    expect(
      tester.getRect(find.byKey(const ValueKey('deck-title-artist'))).width,
      56,
      reason: 'the cluster may shrink below 72px before the title gives way',
    );
  });
}
