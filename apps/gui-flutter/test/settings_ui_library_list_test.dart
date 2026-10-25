import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';
import 'package:gui_flutter/widgets/settings/mixxx_import_panel.dart';
import 'package:gui_flutter/widgets/settings/settings_library_panel.dart';
import 'package:gui_flutter/widgets/settings/settings_ui_panel.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:material_ui/material_ui.dart';

import 'support/mixar_material_app.dart';

void main() {
  testWidgets('UI panel hosts library list controls', (tester) async {
    await tester.pumpWidget(
      ProviderScope(
        child: mixarTestMaterialApp(
          theme: MixarThemeData.dark(),
          home: Scaffold(
            body: SingleChildScrollView(
              child: SettingsUiPanel(
                draft: defaultAppSettings(),
                onChanged: (_) {},
              ),
            ),
          ),
        ),
      ),
    );
    expect(find.text('Library list'), findsOneWidget);
    expect(find.text('Dim played tracks'), findsOneWidget);
    expect(find.text('TRACK ROW LAYOUT'), findsOneWidget);
    expect(find.text('Mixer meters'), findsOneWidget);
    expect(find.text('METER MODE'), findsOneWidget);
  }, semanticsEnabled: false);

  testWidgets('UI panel hides meter mode when mixer meters off', (
    tester,
  ) async {
    await tester.pumpWidget(
      ProviderScope(
        child: mixarTestMaterialApp(
          theme: MixarThemeData.dark(),
          home: Scaffold(
            body: SingleChildScrollView(
              child: SettingsUiPanel(
                draft: copyAppSettings(
                  defaultAppSettings(),
                  showMixerMeters: false,
                ),
                onChanged: (_) {},
              ),
            ),
          ),
        ),
      ),
    );
    expect(find.text('Mixer meters'), findsOneWidget);
    expect(find.text('METER MODE'), findsNothing);
  }, semanticsEnabled: false);

  testWidgets('library list toggles which row fields show', (tester) async {
    AppSettings? changed;
    await tester.pumpWidget(
      ProviderScope(
        child: mixarTestMaterialApp(
          theme: MixarThemeData.dark(),
          home: Scaffold(
            body: SingleChildScrollView(
              child: SettingsUiPanel(
                draft: defaultAppSettings(),
                onChanged: (next) => changed = next,
              ),
            ),
          ),
        ),
      ),
    );

    // SettingsField uppercases its label; the checkboxes do not.
    expect(find.text('VISIBLE FIELDS'), findsOneWidget);
    // Every track field is offered, including the drawer-only ones.
    for (final label in ['Album', 'Genre', 'Year', 'Track #', 'Disc #']) {
      expect(find.text(label), findsOneWidget, reason: label);
    }

    // Turning one on appends it, so it gives up its space first when the pane
    // is narrow.
    await tester.ensureVisible(find.text('ISRC'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('ISRC'));
    await tester.pumpAndSettle();
    expect(changed?.libraryRowFields, [
      ...kDefaultLibraryRowFields,
      LibraryRowFieldSetting.isrc,
    ]);

    // Turning one off removes it and keeps the rest of the order.
    await tester.pumpWidget(
      ProviderScope(
        child: mixarTestMaterialApp(
          theme: MixarThemeData.dark(),
          home: Scaffold(
            body: SingleChildScrollView(
              child: SettingsUiPanel(
                draft: copyAppSettings(
                  defaultAppSettings(),
                  libraryRowFields: const [
                    ...kDefaultLibraryRowFields,
                    LibraryRowFieldSetting.isrc,
                  ],
                ),
                onChanged: (next) => changed = next,
              ),
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();
    await tester.tap(find.text('Album'));
    await tester.pumpAndSettle();
    expect(
      changed?.libraryRowFields,
      isNot(contains(LibraryRowFieldSetting.album)),
    );
    expect(changed?.libraryRowFields.first, LibraryRowFieldSetting.genre);
  });

  testWidgets('Library panel no longer hosts list controls', (tester) async {
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          mixxxDatabasePathProvider.overrideWith((ref) async => null),
        ],
        child: mixarTestMaterialApp(
          theme: MixarThemeData.dark(),
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
    await tester.pumpAndSettle();
    expect(find.text('Dim played tracks'), findsNothing);
    expect(find.text('TRACK ROW LAYOUT'), findsNothing);
  }, semanticsEnabled: false);
}
