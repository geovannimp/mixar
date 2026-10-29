import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/theme/mixar_colors.dart';
import 'package:gui_flutter/core/theme/mixar_palette.dart';
import 'package:gui_flutter/core/utils/desktop.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/providers/settings/settings_providers.dart';
import 'package:gui_flutter/providers/shell/controller_providers.dart';
import 'package:gui_flutter/screens/app_shell.dart';
import 'package:gui_flutter/screens/application.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:material_ui/material_ui.dart';

void main() {
  // Application builds DesktopChrome, which touches window_manager when the
  // host looks like a desktop. Widget tests have no window manager.
  setUp(() => debugOverrideDesktopWindow = false);
  tearDown(() => debugOverrideDesktopWindow = null);

  Future<void> pumpApp(WidgetTester tester, AppSettings settings) async {
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          appSettingsProvider.overrideWith((ref) async => settings),
          audioDevicesProvider.overrideWith((ref) async => const []),
          samplerBanksProvider.overrideWith((ref) async => const []),
          controllerTransportProvider.overrideWith((ref) async => null),
        ],
        child: const Application(appTitle: 'Mixar'),
      ),
    );
    // The settings future resolves in a microtask: one frame publishes it, the
    // second rebuilds from the resolved value.
    await tester.pump();
    await tester.pump();
  }

  for (final (setting, mode) in [
    (ThemeSetting.light, ThemeMode.light),
    (ThemeSetting.dark, ThemeMode.dark),
    (ThemeSetting.system, ThemeMode.system),
  ]) {
    testWidgets(
      'theme ${setting.name} sets MaterialApp.themeMode ${mode.name}',
      (tester) async {
        await pumpApp(
          tester,
          copyAppSettings(defaultAppSettings(), theme: setting),
        );

        expect(
          tester.widget<MaterialApp>(find.byType(MaterialApp)).themeMode,
          mode,
        );
      },
    );
  }

  testWidgets('light theme reaches Mixar shell tokens', (tester) async {
    await pumpApp(
      tester,
      copyAppSettings(defaultAppSettings(), theme: ThemeSetting.light),
    );

    final shell = tester.element(find.byType(AppShell));
    expect(Theme.brightnessOf(shell), Brightness.light);
    expect(MixarTheme.of(shell).colors.brightness, Brightness.light);
  });

  testWidgets('default boot keeps the teal accent', (tester) async {
    await pumpApp(tester, defaultAppSettings());

    final shell = tester.element(find.byType(AppShell));
    final colors = MixarTheme.of(shell).colors;
    expect(colors.brightness, Brightness.light);
    expect(colors.primary, MixarTeal.s600);
  });

  testWidgets('saved violet palette re-tints the shell in dark mode', (
    tester,
  ) async {
    await pumpApp(
      tester,
      copyAppSettings(
        defaultAppSettings(),
        theme: ThemeSetting.dark,
        accentPalette: AccentPaletteSetting.violet,
      ),
    );
    // Themed root animates the ThemeData swap; let it settle before reading.
    await tester.pumpAndSettle();

    final shell = tester.element(find.byType(AppShell));
    expect(Theme.brightnessOf(shell), Brightness.dark);
    final colors = MixarTheme.of(shell).colors;
    expect(colors.primary, MixarViolet.s400);
    expect(colors.selection, MixarViolet.s950);
    expect(colors.border, MixarColors.dark.border);
  });
}
