import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/theme/material_theme.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/providers/settings/settings_providers.dart';
import 'package:gui_flutter/widgets/shell/app_tooltip.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:hint_kit/hint_kit.dart';
import 'package:material_ui/material_ui.dart';

import 'support/mixar_material_app.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  Future<void> pumpTip(
    WidgetTester tester, {
    required bool showTooltips,
    String? description,
    MixarThemeData? theme,
  }) async {
    theme ??= MixarThemeData.dark();
    final settings = copyAppSettings(
      defaultAppSettings(),
      showTooltips: showTooltips,
    );
    await tester.pumpWidget(
      ProviderScope(
        overrides: [appSettingsProvider.overrideWith((ref) async => settings)],
        child: MaterialApp(
          theme: materialUiThemeFromMixar(theme),
          builder: mixarMaterialAppBuilder(theme),
          home: Scaffold(
            body: AppTooltip(
              tip: 'Play',
              description: description,
              child: const Text('child'),
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();
  }

  testWidgets('wraps child in Hint when showTooltips is on', (tester) async {
    await pumpTip(tester, showTooltips: true);
    expect(find.byType(Hint), findsOneWidget);
    expect(find.text('child'), findsOneWidget);
  });

  testWidgets('skips Hint when showTooltips is off', (tester) async {
    await pumpTip(tester, showTooltips: false);
    expect(find.byType(Hint), findsNothing);
    expect(find.text('child'), findsOneWidget);
  });

  testWidgets('accepts an optional description', (tester) async {
    await pumpTip(
      tester,
      showTooltips: true,
      description: 'Keeps pitch when changing tempo.',
    );
    expect(find.byType(Hint), findsOneWidget);
  });

  for (final theme in [MixarThemeData.dark(), MixarThemeData.light()]) {
    testWidgets('bubble follows the ${theme.colors.brightness.name} theme '
        'with a hairline border', (tester) async {
      await pumpTip(tester, showTooltips: true, theme: theme);
      final hint = tester.widget<Hint>(find.byType(Hint));
      final colors = theme.colors;
      expect(
        hint.theme!.backgroundColor,
        colors.card,
        reason: 'fill must come from the theme, not a hardcoded white',
      );
      expect(hint.theme!.borderColor, colors.border);
      // hint_kit strokes the silhouette centred, so the chrome hairline is
      // doubled to leave one full-strength rim inside the bubble.
      expect(hint.theme!.borderWidth, theme.style.borderWidth * 2);
      expect(hint.theme!.foregroundColor, colors.foreground);
    });
  }
}
