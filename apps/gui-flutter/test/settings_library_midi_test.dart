import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/providers/shell/controller_providers.dart';
import 'package:gui_flutter/src/rust/api/controller.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';
import 'package:gui_flutter/widgets/settings/settings_controllers_panel.dart';
import 'package:gui_flutter/widgets/settings/settings_header_switch.dart';
import 'package:gui_flutter/widgets/settings/settings_library_panel.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:material_ui/material_ui.dart';

import 'support/mixar_material_app.dart';

void main() {
  testWidgets('library settings shows analysis quality', (tester) async {
    final theme = MixarThemeData.dark();
    await tester.pumpWidget(
      ProviderScope(
        child: mixarTestMaterialApp(
          theme: theme,
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
    final theme = MixarThemeData.dark();
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          controllerTransportProvider.overrideWith((ref) async => null),
          controllerDevicesProvider.overrideWith((ref) async => const []),
        ],
        child: mixarTestMaterialApp(
          theme: theme,
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
    expect(find.text('MIDI ports'), findsOneWidget);
    expect(find.text('No MIDI ports detected.'), findsOneWidget);
  }, semanticsEnabled: false);

  testWidgets('controllers settings lists populated MIDI ports', (
    tester,
  ) async {
    final theme = MixarThemeData.dark();
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
        child: mixarTestMaterialApp(
          theme: theme,
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
    expect(find.text('IN'), findsOneWidget);
    expect(find.text('OUT'), findsOneWidget);
    expect(find.text('DDJ-400'), findsOneWidget);
    expect(find.text('→ pioneer-ddj-400'), findsOneWidget);
    expect(find.text('Virtual Out'), findsOneWidget);
    expect(find.text('No mapping'), findsOneWidget);
  }, semanticsEnabled: false);

  testWidgets('controllers trust toggle updates draft trusted ids', (
    tester,
  ) async {
    const mapping = ControllerMappingInfo(
      id: 'ddj-400',
      deviceId: 'pioneer.ddj-400',
      vendorName: 'Pioneer',
      productName: 'DDJ-400',
      updateAvailable: false,
      midiNameContains: ['DDJ-400'],
      attached: false,
    );
    AppSettings? changed;
    final theme = MixarThemeData.dark();
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          controllerTransportProvider.overrideWith((ref) async => null),
          controllerMappingsProvider.overrideWith((ref) async => [mapping]),
          controllerDevicesProvider.overrideWith((ref) async => const []),
        ],
        child: mixarTestMaterialApp(
          theme: theme,
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
    // Negative path: no stored version and no update → no badge and no chip.
    expect(find.text('Update available'), findsNothing);
    expect(find.text('v1.0.0'), findsNothing);
    await tester.tap(find.text('Trust'));
    await tester.pumpAndSettle();
    expect(changed?.trustedControllerDeviceIds, ['pioneer.ddj-400']);
  }, semanticsEnabled: false);

  testWidgets('controllers mapping row shows version and update badge', (
    tester,
  ) async {
    const mapping = ControllerMappingInfo(
      id: 'ddj-400',
      deviceId: 'pioneer.ddj-400',
      vendorName: 'Pioneer',
      productName: 'DDJ-400',
      version: '1.0.0',
      updateAvailable: true,
      midiNameContains: ['DDJ-400'],
      attached: false,
    );
    final theme = MixarThemeData.dark();
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          controllerTransportProvider.overrideWith((ref) async => null),
          controllerMappingsProvider.overrideWith((ref) async => [mapping]),
          controllerDevicesProvider.overrideWith((ref) async => const []),
        ],
        child: mixarTestMaterialApp(
          theme: theme,
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
    expect(find.text('Update available'), findsOneWidget);
    expect(find.text('Update'), findsOneWidget);
    expect(find.text('Trust'), findsOneWidget);
    expect(find.text('Attach'), findsOneWidget);
    expect(find.textContaining('v1.0.0'), findsOneWidget);
  }, semanticsEnabled: false);

  testWidgets('library settings exposes the tag write-back opt-in', (
    tester,
  ) async {
    final theme = MixarThemeData.dark();
    AppSettings? changed;
    await tester.pumpWidget(
      ProviderScope(
        child: mixarTestMaterialApp(
          theme: theme,
          home: Scaffold(
            body: SingleChildScrollView(
              child: SettingsLibraryPanel(
                draft: defaultAppSettings(),
                onChanged: (next) => changed = next,
              ),
            ),
          ),
        ),
      ),
    );

    expect(find.text('File tags'), findsOneWidget);
    expect(defaultAppSettings().tagWriteBack, isFalse);
    expect(
      tester
          .widget<SettingsHeaderSwitch>(find.byType(SettingsHeaderSwitch))
          .value,
      isFalse,
    );

    // The switch sits in the card header row with the title (like vinyl brake),
    // not as a labelled row in the card body.
    final headerRow = find
        .ancestor(
          of: find.byType(SettingsHeaderSwitch),
          matching: find.byType(Row),
        )
        .first;
    expect(
      find.descendant(of: headerRow, matching: find.text('File tags')),
      findsOneWidget,
    );

    await tester.tap(find.byType(SettingsHeaderSwitch));
    await tester.pumpAndSettle();
    expect(changed?.tagWriteBack, isTrue);

    // The header switch reflects the draft it is given, so the card reads as
    // on when the setting is on.
    await tester.pumpWidget(
      ProviderScope(
        child: mixarTestMaterialApp(
          theme: theme,
          home: Scaffold(
            body: SingleChildScrollView(
              child: SettingsLibraryPanel(
                draft: copyAppSettings(
                  defaultAppSettings(),
                  tagWriteBack: true,
                ),
                onChanged: (_) {},
              ),
            ),
          ),
        ),
      ),
    );
    expect(
      tester
          .widget<SettingsHeaderSwitch>(find.byType(SettingsHeaderSwitch))
          .value,
      isTrue,
    );
  }, semanticsEnabled: false);
}
