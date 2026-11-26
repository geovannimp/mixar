import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/models/bus_enums.dart';
import 'package:gui_flutter/core/theme/deck_accent.dart';
import 'package:gui_flutter/core/theme/material_theme.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/providers/settings/settings_providers.dart';
import 'package:gui_flutter/widgets/mixer/deck_performance_panel.dart';
import 'package:gui_flutter/widgets/shell/m_tab_bar.dart';
import 'package:gui_flutter/widgets/shell/mixar_select.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';
import 'package:material_ui/material_ui.dart';

import 'support/mixar_material_app.dart';

void main() {
  Future<void> pumpPanel(
    WidgetTester tester, {
    Size size = const Size(360, 320),
    DeckAccent accent = DeckAccent.a,
    int? deckId,
  }) async {
    final theme = MixarThemeData.dark();
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          appSettingsProvider.overrideWith((ref) async => defaultAppSettings()),
        ],
        child: MaterialApp(
          theme: materialUiThemeFromMixar(theme),
          builder: mixarMaterialAppBuilder(theme),
          home: Scaffold(
            body: SizedBox(
              width: size.width,
              height: size.height,
              child: DeckPerformancePanel(
                hasTrack: true,
                accent: accent,
                deckId: deckId,
              ),
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();
  }

  testWidgets('Pads / Loop / Grid / Jog tabs switch exclusive content', (
    tester,
  ) async {
    await pumpPanel(tester);

    expect(find.byIcon(LucideIcons.layoutGrid), findsOneWidget);
    expect(find.byIcon(LucideIcons.repeat2), findsOneWidget);
    expect(find.byIcon(LucideIcons.audioLines), findsOneWidget);
    expect(find.byIcon(LucideIcons.disc3), findsOneWidget);
    expect(
      tester.getCenter(find.byIcon(LucideIcons.layoutGrid)).dy <
          tester.getCenter(find.byIcon(LucideIcons.repeat2)).dy,
      isTrue,
    );
    expect(
      tester.getCenter(find.byIcon(LucideIcons.repeat2)).dy <
          tester.getCenter(find.byIcon(LucideIcons.audioLines)).dy,
      isTrue,
    );
    expect(
      tester.getCenter(find.byIcon(LucideIcons.audioLines)).dy <
          tester.getCenter(find.byIcon(LucideIcons.disc3)).dy,
      isTrue,
    );
    expect(find.text('Cue'), findsOneWidget);
    expect(find.text('IN'), findsNothing);
    expect(find.text('Now'), findsNothing);
    expect(find.bySemanticsLabel('Jog wheel'), findsNothing);

    await tester.tap(find.byIcon(LucideIcons.repeat2));
    await tester.pumpAndSettle();
    expect(find.text('IN'), findsOneWidget);
    expect(find.text('OUT'), findsOneWidget);
    expect(find.text('Cue'), findsNothing);
    expect(find.text('Now'), findsNothing);
    expect(find.bySemanticsLabel('Jog wheel'), findsNothing);

    await tester.tap(find.byIcon(LucideIcons.audioLines));
    await tester.pumpAndSettle();
    expect(find.text('Now'), findsOneWidget);
    expect(find.text('Cue'), findsNothing);
    expect(find.text('IN'), findsNothing);
    expect(find.bySemanticsLabel('Jog wheel'), findsNothing);

    await tester.tap(find.byIcon(LucideIcons.disc3));
    await tester.pumpAndSettle();
    expect(find.bySemanticsLabel('Jog wheel'), findsOneWidget);
    expect(find.text('Cue'), findsNothing);
    expect(find.text('IN'), findsNothing);
    expect(find.text('Now'), findsNothing);

    await tester.tap(find.byIcon(LucideIcons.layoutGrid));
    await tester.pumpAndSettle();
    expect(find.text('Cue'), findsOneWidget);
    expect(find.text('IN'), findsNothing);
    expect(find.text('Now'), findsNothing);
    expect(find.bySemanticsLabel('Jog wheel'), findsNothing);
  });

  for (final (accent, side) in [
    (DeckAccent.a, 'left'),
    (DeckAccent.b, 'right'),
    (DeckAccent.d, 'right'),
  ]) {
    testWidgets('${accent.name} deck rail sits on the $side', (tester) async {
      await pumpPanel(tester, size: const Size(370, 250), accent: accent);

      final rail = tester.getRect(find.byKey(kMTabBarKey));
      final select = tester.getRect(find.byType(MixarSelect<PadMode>));
      expect(rail.width, 48);
      if (side == 'left') {
        expect(rail.center.dx, lessThan(select.center.dx));
      } else {
        expect(rail.center.dx, greaterThan(select.center.dx));
      }

      // The hairline follows the rail: it separates the bar from the content
      // and leaves the outer edge bare.
      final theme = MixarThemeData.dark();
      final bar = tester.widget<Container>(find.byKey(kMTabBarKey));
      final border =
          (bar.foregroundDecoration! as BoxDecoration).border! as Border;
      final hairline = BorderSide(
        color: theme.colors.border,
        width: theme.style.borderWidth,
      );
      if (side == 'left') {
        expect(border.right, hairline);
        expect(border.left, BorderSide.none);
      } else {
        expect(border.left, hairline);
        expect(border.right, BorderSide.none);
      }

      // Loop lives in the rail on the same side; pressing it leaves Pads.
      await tester.tap(find.byIcon(LucideIcons.repeat2));
      await tester.pumpAndSettle();
      expect(find.byType(MixarSelect<PadMode>), findsNothing);
      expect(find.text('IN'), findsOneWidget);
    });
  }

  testWidgets('deck id picks the rail side over the preview accent', (
    tester,
  ) async {
    await pumpPanel(tester, size: const Size(370, 250), deckId: 3);

    final rail = tester.getRect(find.byKey(kMTabBarKey));
    final select = tester.getRect(find.byType(MixarSelect<PadMode>));
    expect(rail.center.dx, greaterThan(select.center.dx));
  });

  testWidgets('the shell paints no outer frame', (tester) async {
    await pumpPanel(tester, size: const Size(370, 250));

    final shell = tester.widget<Container>(
      find
          .descendant(
            of: find.byType(DeckPerformancePanel),
            matching: find.byType(Container),
          )
          .first,
    );
    expect(shell.foregroundDecoration, isNull);
    final decoration = shell.decoration! as BoxDecoration;
    expect(decoration.border, isNull);
    expect(
      decoration.color,
      MixarThemeData.dark().colors.secondary,
      reason: 'the shell paints the darker gray plate',
    );
  });
}
