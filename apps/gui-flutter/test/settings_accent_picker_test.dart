import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';
import 'package:gui_flutter/widgets/settings/settings_accent_picker.dart';
import 'package:gui_flutter/widgets/settings/settings_ui_panel.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:material_ui/material_ui.dart';

import 'support/mixar_material_app.dart';

void main() {
  Future<void> pumpPanel(
    WidgetTester tester, {
    AppSettings? draft,
    ValueChanged<AppSettings>? onChanged,
  }) async {
    await tester.pumpWidget(
      ProviderScope(
        child: mixarTestMaterialApp(
          theme: MixarThemeData.dark(),
          home: Scaffold(
            body: SingleChildScrollView(
              child: SettingsUiPanel(
                draft: draft ?? defaultAppSettings(),
                onChanged: onChanged ?? (_) {},
              ),
            ),
          ),
        ),
      ),
    );
  }

  testWidgets('accent picker lists all five palettes in one row, teal marked', (
    tester,
  ) async {
    final handle = tester.ensureSemantics();
    await pumpPanel(tester);
    await tester.pump();

    expect(find.text('ACCENT COLOR'), findsOneWidget);
    expect(find.byType(SettingsAccentPicker), findsOneWidget);
    for (final label in ['Teal', 'Blue', 'Violet', 'Orange', 'Pink']) {
      expect(find.text(label), findsOneWidget, reason: label);
    }
    expect(find.text('Emerald'), findsNothing);

    // One row: every swatch centre shares a baseline.
    final centres = {
      for (final label in ['Teal', 'Blue', 'Violet', 'Orange', 'Pink'])
        tester.getCenter(find.bySemanticsLabel(label)).dy.round(),
    };
    expect(centres, hasLength(1));

    // One node per cell: the visible label is excluded from semantics, so the
    // node label is exactly the palette name (no duplicate announcement).
    expect(
      tester.getSemantics(find.bySemanticsLabel('Teal')),
      isSemantics(label: 'Teal', isSelected: true, isButton: true),
    );
    expect(
      tester.getSemantics(find.bySemanticsLabel('Violet')),
      isSemantics(label: 'Violet', isSelected: false, isButton: true),
    );
    handle.dispose();
  });

  testWidgets('tapping Violet emits the palette and leaves other fields', (
    tester,
  ) async {
    final baseline = defaultAppSettings();
    AppSettings? changed;
    await pumpPanel(
      tester,
      draft: baseline,
      onChanged: (settings) => changed = settings,
    );

    await tester.tap(find.text('Violet'));
    await tester.pump();

    expect(changed?.accentPalette, AccentPaletteSetting.violet);
    expect(changed?.theme, baseline.theme);
    expect(changed?.libraryRowFields, baseline.libraryRowFields);
    expect(changed?.showTooltips, baseline.showTooltips);
  }, semanticsEnabled: false);

  testWidgets('a narrow card shrinks the swatches instead of overflowing', (
    tester,
  ) async {
    await tester.pumpWidget(
      ProviderScope(
        child: mixarTestMaterialApp(
          theme: MixarThemeData.dark(),
          home: Scaffold(
            body: Align(
              alignment: Alignment.topLeft,
              child: SizedBox(
                width: 140,
                child: SettingsAccentPicker(
                  value: AccentPaletteSetting.teal,
                  onChanged: (_) {},
                ),
              ),
            ),
          ),
        ),
      ),
    );

    expect(tester.takeException(), isNull);
    final swatch = tester.getSize(
      find
          .descendant(
            of: find.byType(SettingsAccentPicker),
            matching: find.byType(AspectRatio),
          )
          .first,
    );
    expect(swatch.width, lessThan(22));
    expect(swatch.width, swatch.height);
  }, semanticsEnabled: false);
}
