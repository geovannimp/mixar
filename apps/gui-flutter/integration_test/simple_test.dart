import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/utils/desktop.dart';
import 'package:gui_flutter/providers/settings/settings_providers.dart';
import 'package:gui_flutter/screens/app_shell.dart';
import 'package:gui_flutter/screens/application.dart';
import 'package:gui_flutter/src/rust/api/meta.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';
import 'package:gui_flutter/src/rust/frb_generated.dart';
import 'package:integration_test/integration_test.dart';
import 'package:material_ui/material_ui.dart';

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();
  setUpAll(() async {
    debugOverrideDesktopWindow = false;
    await RustLib.init();
  });
  testWidgets('shows shared app title from Rust', (tester) async {
    final title = appDisplayName();
    // The header draws the logo with the app title as its semantic label; there
    // is no text widget carrying it.
    final semantics = tester.ensureSemantics();
    await tester.pumpWidget(ProviderScope(child: Application(appTitle: title)));
    expect(find.bySemanticsLabel(title), findsWidgets);
    semantics.dispose();
  });

  testWidgets('the first frame already carries the saved theme', (
    tester,
  ) async {
    // Same boot as main(): resolve settings in the container before the tree
    // exists, so frame one is not the `system` default that a saved light theme
    // would then replace (the visible dark blink).
    final container = ProviderContainer();
    addTearDown(container.dispose);
    await preloadAppSettings(container);
    // Whatever this profile has saved (including `system`), the preload must
    // have landed before the tree exists: a still-pending read is what renders
    // the `system` fallback on frame one. A timeout here is the regression, not
    // noise -- it means the saved theme misses the first frame.
    final preloaded = container.read(appSettingsProvider);
    expect(
      preloaded.isLoading,
      isFalse,
      reason:
          'preload did not land before the first frame; frame one would show '
          'the system theme instead of the saved one',
    );
    final saved = preloaded.requireValue;

    await tester.pumpWidget(
      UncontrolledProviderScope(
        container: container,
        child: Application(appTitle: appDisplayName()),
      ),
    );

    expect(
      tester.widget<MaterialApp>(find.byType(MaterialApp)).themeMode,
      switch (saved.theme) {
        ThemeSetting.system => ThemeMode.system,
        ThemeSetting.light => ThemeMode.light,
        ThemeSetting.dark => ThemeMode.dark,
      },
    );
    final shell = tester.element(find.byType(AppShell));
    if (saved.theme != ThemeSetting.system) {
      expect(
        Theme.brightnessOf(shell),
        saved.theme == ThemeSetting.dark ? Brightness.dark : Brightness.light,
      );
    }
  });
}
