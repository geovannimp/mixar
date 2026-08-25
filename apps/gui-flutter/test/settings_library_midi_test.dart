import 'package:gui_flutter/shell/material_theme.dart';
import 'package:material_ui/material_ui.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:forui/forui.dart';
import 'package:gui_flutter/settings/settings_controllers_panel.dart';
import 'package:gui_flutter/settings/settings_defaults.dart';
import 'package:gui_flutter/settings/settings_library_panel.dart';
import 'package:gui_flutter/shell/controller_providers.dart';
import 'package:gui_flutter/src/rust/api/controller.dart';

void main() {
  testWidgets('library settings shows analysis quality', (tester) async {
    final theme = FTheme.neutral.dark.desktop;
    await tester.pumpWidget(
      ProviderScope(
        child: MaterialApp(
          theme: materialUiThemeFromForui(theme),
          builder: (context, child) => MaterialUiCompatibilityBridge(
            // ignore: deprecated_member_use
            child: FTheme(data: theme, child: child!),
          ),
          home: Scaffold(
            body: SettingsLibraryPanel(
              draft: defaultAppSettings(),
              onChanged: (_) {},
            ),
          ),
        ),
      ),
    );
    expect(find.text('ANALYSIS QUALITY'), findsOneWidget);
    expect(find.text('KEY DISPLAY MODE'), findsOneWidget);
  }, semanticsEnabled: false);

  testWidgets('controllers settings lists MIDI ports', (tester) async {
    final theme = FTheme.neutral.dark.desktop;
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          controllerTransportProvider.overrideWith((ref) async => null),
          controllerDevicesProvider.overrideWith((ref) async => const []),
        ],
        child: MaterialApp(
          theme: materialUiThemeFromForui(theme),
          builder: (context, child) => MaterialUiCompatibilityBridge(
            // ignore: deprecated_member_use
            child: FTheme(data: theme, child: child!),
          ),
          home: Scaffold(
            body: SettingsControllersPanel(
              draft: defaultAppSettings(),
              onChanged: (_) {},
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();
    expect(find.text('MIDI PORTS'), findsOneWidget);
    expect(find.text('No MIDI ports detected.'), findsOneWidget);
  }, semanticsEnabled: false);

  testWidgets('controllers settings lists populated MIDI ports', (
    tester,
  ) async {
    final theme = FTheme.neutral.dark.desktop;
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          controllerTransportProvider.overrideWith((ref) async => null),
          controllerDevicesProvider.overrideWith(
            (ref) async => const [
              ControllerDeviceInfo(
                portName: 'DDJ-400',
                direction: ControllerDeviceDirection.input,
                matchedMappingId: 'pioneer-ddj-400',
              ),
              ControllerDeviceInfo(
                portName: 'Virtual Out',
                direction: ControllerDeviceDirection.output,
              ),
            ],
          ),
        ],
        child: MaterialApp(
          theme: materialUiThemeFromForui(theme),
          builder: (context, child) => MaterialUiCompatibilityBridge(
            // ignore: deprecated_member_use
            child: FTheme(data: theme, child: child!),
          ),
          home: Scaffold(
            body: SettingsControllersPanel(
              draft: defaultAppSettings(),
              onChanged: (_) {},
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();
    expect(find.text('input DDJ-400 → pioneer-ddj-400'), findsOneWidget);
    expect(find.text('output Virtual Out'), findsOneWidget);
  }, semanticsEnabled: false);
}
