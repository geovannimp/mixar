import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_riverpod/misc.dart' show Override;
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

/// Providers `Application` needs stubbed in a widget test. Shared by the
/// pumping helpers below so a new dependency is stubbed once.
List<Override> appStubs(AppSettings settings) => [
  appSettingsProvider.overrideWith((ref) async => settings),
  audioDevicesProvider.overrideWith((ref) async => const []),
  samplerBanksProvider.overrideWith((ref) async => const []),
  controllerTransportProvider.overrideWith((ref) async => null),
];

void main() {
  // Application builds DesktopChrome, which touches window_manager when the
  // host looks like a desktop. Widget tests have no window manager.
  setUp(() => debugOverrideDesktopWindow = false);
  tearDown(() => debugOverrideDesktopWindow = null);

  Future<void> pumpApp(WidgetTester tester, AppSettings settings) async {
    await tester.pumpWidget(
      ProviderScope(
        overrides: appStubs(settings),
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

  for (final (saved, os) in [
    (ThemeSetting.light, Brightness.dark),
    (ThemeSetting.dark, Brightness.light),
  ]) {
    testWidgets(
      'preloaded ${saved.name} theme beats a ${os.name} OS on the first frame',
      (tester) async {
        // The `system` fallback would show [os] here, so this pins that the
        // saved theme is what frame one renders instead of flipping after the
        // bridge answers (the boot flash).
        tester.platformDispatcher.platformBrightnessTestValue = os;
        addTearDown(tester.platformDispatcher.clearPlatformBrightnessTestValue);

        final settings = copyAppSettings(defaultAppSettings(), theme: saved);
        final container = ProviderContainer(overrides: appStubs(settings));
        addTearDown(container.dispose);
        await preloadAppSettings(container);

        await tester.pumpWidget(
          UncontrolledProviderScope(
            container: container,
            child: const Application(appTitle: 'Mixar'),
          ),
        );

        // No extra pump: the frame building here is the one the user sees.
        expect(
          tester.widget<MaterialApp>(find.byType(MaterialApp)).themeMode,
          saved == ThemeSetting.light ? ThemeMode.light : ThemeMode.dark,
        );
        expect(
          Theme.brightnessOf(tester.element(find.byType(AppShell))),
          saved == ThemeSetting.light ? Brightness.light : Brightness.dark,
        );
        await tester.pumpAndSettle();
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
    // teal-700, not teal-600: the shared token has to pass AA on bone-100
    // (4.71:1 as text, 5.47:1 white-on-fill).
    expect(colors.primary, MixarTeal.s700);
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
