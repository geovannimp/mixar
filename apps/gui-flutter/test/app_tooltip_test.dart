import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:forui/forui.dart';
import 'package:gui_flutter/settings/settings_defaults.dart';
import 'package:gui_flutter/settings/settings_providers.dart';
import 'package:gui_flutter/shell/app_tooltip.dart';
import 'package:gui_flutter/shell/material_theme.dart';
import 'package:material_ui/material_ui.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  Future<void> pumpTip(
    WidgetTester tester, {
    required bool showTooltips,
    String? description,
  }) async {
    final theme = FTheme.neutral.dark.desktop;
    final settings = copyAppSettings(
      defaultAppSettings(),
      showTooltips: showTooltips,
    );
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          appSettingsProvider.overrideWith((ref) async => settings),
        ],
        child: MaterialApp(
          theme: materialUiThemeFromForui(theme),
          builder: (context, child) => MaterialUiCompatibilityBridge(
            // ignore: deprecated_member_use
            child: FTheme(
              data: theme,
              child: FTooltipGroup(child: child!),
            ),
          ),
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

  testWidgets('wraps child in FTooltip when showTooltips is on', (tester) async {
    await pumpTip(tester, showTooltips: true);
    expect(find.byType(FTooltip), findsOneWidget);
    expect(find.text('child'), findsOneWidget);
  });

  testWidgets('skips FTooltip when showTooltips is off', (tester) async {
    await pumpTip(tester, showTooltips: false);
    expect(find.byType(FTooltip), findsNothing);
    expect(find.text('child'), findsOneWidget);
  });

  testWidgets('accepts an optional description', (tester) async {
    await pumpTip(
      tester,
      showTooltips: true,
      description: 'Keeps pitch when changing tempo.',
    );
    expect(find.byType(FTooltip), findsOneWidget);
  });
}
