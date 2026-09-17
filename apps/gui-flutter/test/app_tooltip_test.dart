import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/settings/settings_defaults.dart';
import 'package:gui_flutter/settings/settings_providers.dart';
import 'package:gui_flutter/shell/app_tooltip.dart';
import 'package:gui_flutter/shell/material_theme.dart';
import 'package:gui_flutter/shell/mixar_theme.dart';
import 'package:hint_kit/hint_kit.dart';
import 'package:material_ui/material_ui.dart';

import 'support/mixar_material_app.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  Future<void> pumpTip(
    WidgetTester tester, {
    required bool showTooltips,
    String? description,
  }) async {
    final theme = MixarThemeData.dark();
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
}
