import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';
import 'package:gui_flutter/widgets/settings/mixxx_import_panel.dart';
import 'package:gui_flutter/widgets/settings/settings_library_panel.dart';
import 'package:gui_flutter/widgets/settings/settings_section_card.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:material_ui/material_ui.dart';

import 'support/mixar_material_app.dart';

void main() {
  testWidgets('library settings hosts the visible fields card', (tester) async {
    AppSettings? changed;

    Future<void> pumpPanel(AppSettings draft) async {
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
                  draft: draft,
                  onChanged: (next) => changed = next,
                ),
              ),
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();
    }

    await pumpPanel(defaultAppSettings());

    // Its own card in the Library section, title-only.
    expect(
      find.widgetWithText(SettingsSectionCard, 'Visible fields'),
      findsOneWidget,
    );
    final card = find.ancestor(
      of: find.text('Visible fields'),
      matching: find.byType(SettingsSectionCard),
    );
    expect(
      find.descendant(of: card, matching: find.byType(Text)),
      findsNWidgets(13),
      reason: 'card title plus one label per field, no description line',
    );
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
    await pumpPanel(
      copyAppSettings(
        defaultAppSettings(),
        libraryRowFields: const [
          ...kDefaultLibraryRowFields,
          LibraryRowFieldSetting.isrc,
        ],
      ),
    );
    await tester.ensureVisible(find.text('Album'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Album'));
    await tester.pumpAndSettle();
    expect(
      changed?.libraryRowFields,
      isNot(contains(LibraryRowFieldSetting.album)),
    );
    expect(changed?.libraryRowFields.first, LibraryRowFieldSetting.genre);
  }, semanticsEnabled: false);
}
