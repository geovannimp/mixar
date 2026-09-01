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
import 'package:gui_flutter/src/rust/api/settings.dart';
import 'support/forui_material_app.dart';

void main() {
  testWidgets('library settings shows analysis quality', (tester) async {
    final theme = FTheme.neutral.dark.desktop;
    await tester.pumpWidget(
      ProviderScope(
        child: MaterialApp(
          theme: materialUiThemeFromForui(theme),
          builder: foruiMaterialAppBuilder(theme),
          home: Scaffold(
            body: SingleChildScrollView(
              child: SettingsLibraryPanel(
                draft: defaultAppSettings(),
                onChanged: (_) {},
              ),
            ),
          ),
        ),
      ),
    );
    expect(find.text('ANALYSIS QUALITY'), findsOneWidget);
    expect(find.text('Musical key'), findsOneWidget);
    expect(find.text('KEY DISPLAY MODE'), findsOneWidget);
    expect(find.text('KEY COLOR MODE'), findsOneWidget);
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
          builder: foruiMaterialAppBuilder(theme),
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
          builder: foruiMaterialAppBuilder(theme),
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

  testWidgets('controllers trust toggle updates draft trusted ids', (
    tester,
  ) async {
    const mapping = ControllerMappingInfo(
      id: 'ddj-400',
      deviceId: 'pioneer.ddj-400',
      vendorName: 'Pioneer',
      productName: 'DDJ-400',
      description: null,
      midiNameContains: ['DDJ-400'],
      attached: false,
    );
    AppSettings? changed;
    final theme = FTheme.neutral.dark.desktop;
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          controllerTransportProvider.overrideWith((ref) async => null),
          controllerMappingsProvider.overrideWith((ref) async => [mapping]),
          controllerDevicesProvider.overrideWith((ref) async => const []),
        ],
        child: MaterialApp(
          theme: materialUiThemeFromForui(theme),
          builder: foruiMaterialAppBuilder(theme),
          home: Scaffold(
            body: SettingsControllersPanel(
              draft: defaultAppSettings(),
              onChanged: (next) => changed = next,
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();
    await tester.tap(find.byType(FSwitch).first);
    await tester.pumpAndSettle();
    expect(changed?.trustedControllerDeviceIds, ['pioneer.ddj-400']);
  }, semanticsEnabled: false);
}
