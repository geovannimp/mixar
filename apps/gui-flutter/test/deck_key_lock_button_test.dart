import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:forui/forui.dart';
import 'package:gui_flutter/mixer/deck_track_info.dart';
import 'package:gui_flutter/settings/settings_defaults.dart';
import 'package:gui_flutter/settings/settings_providers.dart';
import 'package:gui_flutter/shell/material_theme.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';
import 'package:material_ui/material_ui.dart';
import 'support/forui_material_app.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  Future<void> pumpButton(
    WidgetTester tester, {
    required bool keyLock,
    bool enabled = true,
    VoidCallback? onToggle,
  }) async {
    final theme = FTheme.neutral.dark.desktop;
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          appSettingsProvider.overrideWith((ref) async => defaultAppSettings()),
        ],
        child: MaterialApp(
          theme: materialUiThemeFromForui(theme),
          builder: foruiMaterialAppBuilder(theme),
          home: Scaffold(
            body: DeckKeyLockButton(
              keyLabel: 'F#m',
              keyLock: keyLock,
              enabled: enabled,
              onToggle: onToggle ?? () {},
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();
  }

  testWidgets('shows lock when key lock on and lockOpen when off', (
    tester,
  ) async {
    await pumpButton(tester, keyLock: true);
    expect(find.text('F#m'), findsOneWidget);
    expect(find.byIcon(LucideIcons.lock), findsOneWidget);
    expect(find.byIcon(LucideIcons.lockOpen), findsNothing);

    await pumpButton(tester, keyLock: false);
    expect(find.byIcon(LucideIcons.lockOpen), findsOneWidget);
    expect(find.byIcon(LucideIcons.lock), findsNothing);
  });

  testWidgets('whole button toggles when enabled', (tester) async {
    var taps = 0;
    await pumpButton(tester, keyLock: false, onToggle: () => taps++);

    await tester.tap(find.text('F#m'));
    await tester.pumpAndSettle();
    expect(taps, 1);

    await tester.tap(find.byIcon(LucideIcons.lockOpen));
    await tester.pumpAndSettle();
    expect(taps, 2);
  });

  testWidgets('disabled button does not toggle', (tester) async {
    var taps = 0;
    await pumpButton(
      tester,
      keyLock: false,
      enabled: false,
      onToggle: () => taps++,
    );

    await tester.tap(find.text('F#m'));
    await tester.pumpAndSettle();
    expect(taps, 0);
  });
}
