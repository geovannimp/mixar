import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/models/settings_section.dart';
import 'package:gui_flutter/l10n/app_localizations.dart';
import 'package:gui_flutter/widgets/settings/settings_sidebar.dart';
import 'package:gui_flutter/widgets/shell/m_tappable.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:material_ui/material_ui.dart';

import 'support/mixar_material_app.dart';

void main() {
  Future<void> pumpSidebar(
    WidgetTester tester, {
    required SettingsSection active,
    required ValueChanged<SettingsSection> onSelect,
  }) async {
    final theme = MixarThemeData.dark();
    await tester.pumpWidget(
      mixarTestMaterialApp(
        theme: theme,
        home: Scaffold(
          body: SettingsSidebar(active: active, onSelect: onSelect),
        ),
      ),
    );
  }

  testWidgets('lists section labels and reports onSelect', (tester) async {
    SettingsSection? selected;
    await pumpSidebar(
      tester,
      active: SettingsSection.audio,
      onSelect: (section) => selected = section,
    );

    final l10n = lookupAppLocalizations(const Locale('en'));
    for (final section in kSettingsSections) {
      expect(find.text(section.label(l10n)), findsOneWidget);
    }

    await tester.tap(find.text(SettingsSection.mixer.label(l10n)));
    await tester.pump();
    expect(selected, SettingsSection.mixer);
  });

  testWidgets('selected row tints only its icon with the accent', (
    tester,
  ) async {
    final theme = MixarThemeData.dark();
    await pumpSidebar(tester, active: SettingsSection.ui, onSelect: (_) {});

    final icons = tester.widgetList<Icon>(find.byType(Icon)).toList();
    expect(icons, hasLength(kSettingsSections.length));
    expect(
      icons.where((icon) => icon.color == theme.colors.primary),
      hasLength(1),
    );
    expect(
      icons.where((icon) => icon.color == theme.colors.selectionForeground),
      isEmpty,
    );
    expect(
      icons.where((icon) => icon.color == theme.colors.mutedForeground),
      hasLength(kSettingsSections.length - 1),
    );
  });

  testWidgets('rows are roomy and span the sidebar', (tester) async {
    await pumpSidebar(tester, active: SettingsSection.ui, onSelect: (_) {});

    final sidebar = tester.getRect(find.byType(SettingsSidebar));
    final rows = find.byType(MTappable);
    expect(rows, findsNWidgets(kSettingsSections.length));
    for (var i = 0; i < kSettingsSections.length; i++) {
      final row = tester.getRect(rows.at(i));
      expect(row.width, sidebar.width, reason: 'the row fill spans the pane');
      // Taller than a default chrome row: the long nav list needs air between
      // labels, not a compact strip.
      expect(row.height, greaterThanOrEqualTo(40));
    }

    // Side inset, icon, then the gap before the label.
    expect(tester.getTopLeft(find.text('UI')).dx - sidebar.left, 38);
  });
}
