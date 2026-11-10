import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/models/settings_section.dart';
import 'package:gui_flutter/l10n/app_localizations.dart';
import 'package:gui_flutter/widgets/settings/settings_sidebar.dart';
import 'package:gui_flutter/widgets/shell/m_tappable.dart';
import 'package:gui_flutter/widgets/shell/mixar_input.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';
import 'package:material_ui/material_ui.dart';

import 'support/mixar_material_app.dart';

void main() {
  Future<void> pumpSidebar(
    WidgetTester tester, {
    required SettingsSection active,
    required ValueChanged<SettingsSection> onSelect,
    Locale locale = const Locale('en'),
  }) async {
    final theme = MixarThemeData.dark();
    await tester.pumpWidget(
      mixarTestMaterialApp(
        theme: theme,
        locale: locale,
        home: Scaffold(
          body: SettingsSidebar(
            active: active,
            onSelect: onSelect,
            onQueryChanged: (_) {},
          ),
        ),
      ),
    );
  }

  /// Pumps a sidebar that owns its active section, like the settings page does.
  ///
  /// Returns a getter for the live section so a test can follow search-driven
  /// moves and row picks across rebuilds.
  Future<ValueGetter<SettingsSection>> pumpStatefulSidebar(
    WidgetTester tester, {
    SettingsSection initial = SettingsSection.audio,
  }) async {
    var active = initial;
    await tester.pumpWidget(
      mixarTestMaterialApp(
        theme: MixarThemeData.dark(),
        home: Scaffold(
          body: StatefulBuilder(
            builder: (context, setState) => SettingsSidebar(
              active: active,
              onSelect: (section) => setState(() => active = section),
              onQueryChanged: (_) {},
            ),
          ),
        ),
      ),
    );
    return () => active;
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

  testWidgets('search field is first and filters sections by name', (
    tester,
  ) async {
    SettingsSection? selected;
    await pumpSidebar(
      tester,
      active: SettingsSection.audio,
      onSelect: (section) => selected = section,
    );
    final l10n = lookupAppLocalizations(const Locale('en'));

    final field = find.byType(MixarInput);
    expect(field, findsOneWidget);
    expect(
      tester.getTopLeft(field).dy,
      lessThan(tester.getTopLeft(find.text(SettingsSection.ui.label(l10n))).dy),
    );

    await tester.enterText(field, 'theme');
    await tester.pump();
    // Audio was active and is hidden by the query, so the first match is
    // selected automatically.
    expect(selected, SettingsSection.ui);
    expect(find.text(SettingsSection.ui.label(l10n)), findsOneWidget);
    expect(find.text(SettingsSection.audio.label(l10n)), findsNothing);
    expect(find.text(SettingsSection.privacy.label(l10n)), findsNothing);

    await tester.tap(find.text(SettingsSection.ui.label(l10n)));
    await tester.pump();
    expect(selected, SettingsSection.ui);
    // Clicking a menu item keeps the query and the filtered list.
    expect(tester.widget<MixarInput>(field).controller?.text, 'theme');
    expect(find.text(SettingsSection.audio.label(l10n)), findsNothing);
    expect(find.text(SettingsSection.privacy.label(l10n)), findsNothing);
  });

  testWidgets('search band is flush with the nav and marks focus', (
    tester,
  ) async {
    await pumpSidebar(tester, active: SettingsSection.ui, onSelect: (_) {});
    final theme = MixarThemeData.dark();
    final band = find.byKey(kSettingsSearchBandKey);
    BoxDecoration bandDecoration() =>
        tester.widget<DecoratedBox>(band).decoration as BoxDecoration;

    expect(
      tester.getRect(band).width,
      tester.getRect(find.byType(SettingsSidebar)).width,
    );
    // Taller than a nav row: the filter is the column's primary control.
    expect(
      tester.getRect(band).height,
      greaterThanOrEqualTo(tester.getRect(find.byType(MTappable).first).height),
    );
    expect(bandDecoration().borderRadius, isNull);
    expect(bandDecoration().color, theme.colors.secondary);
    expect(
      (bandDecoration().border! as Border).bottom.color,
      theme.colors.border,
    );

    // Typing focuses the field: the band lifts one solid step while the
    // hairline stays the sidebar's neutral divider — no accent underline.
    await tester.enterText(find.byType(MixarInput), 'theme');
    await tester.pump();

    expect(bandDecoration().color, theme.colors.card);
    expect(
      (bandDecoration().border! as Border).bottom.color,
      theme.colors.border,
    );
  });

  testWidgets('clearing the search goes back to the section it hid', (
    tester,
  ) async {
    final active = await pumpStatefulSidebar(tester);

    // 'theme' hides Audio, so the sidebar auto-selects UI to keep the panel in
    // sync with the filtered list.
    await tester.enterText(find.byType(MixarInput), 'theme');
    await tester.pump();
    expect(active(), SettingsSection.ui);

    // Clearing puts the user back in the section the search took them out of.
    await tester.tap(find.byIcon(LucideIcons.x));
    await tester.pump();
    expect(active(), SettingsSection.audio);
  });

  testWidgets('a row picked while searching survives the clear', (
    tester,
  ) async {
    final active = await pumpStatefulSidebar(tester);
    final l10n = lookupAppLocalizations(const Locale('en'));

    await tester.enterText(find.byType(MixarInput), 'theme');
    await tester.pump();
    await tester.tap(find.text(SettingsSection.ui.label(l10n)));
    await tester.pump();

    await tester.tap(find.byIcon(LucideIcons.x));
    await tester.pump();
    expect(active(), SettingsSection.ui);
  });

  testWidgets(
    'auto-selects the first match when the active section is hidden',
    (tester) async {
      SettingsSection? selected;
      await pumpSidebar(
        tester,
        active: SettingsSection.privacy,
        onSelect: (section) => selected = section,
      );

      await tester.enterText(find.byType(MixarInput), 'theme');
      await tester.pump();
      expect(selected, SettingsSection.ui);
    },
  );

  testWidgets('keeps the active section when it still matches', (tester) async {
    SettingsSection? selected;
    await pumpSidebar(
      tester,
      active: SettingsSection.ui,
      onSelect: (section) => selected = section,
    );
    final l10n = lookupAppLocalizations(const Locale('en'));

    await tester.enterText(find.byType(MixarInput), 'theme');
    await tester.pump();
    expect(selected, isNull);
    expect(find.text(SettingsSection.ui.label(l10n)), findsOneWidget);
  });

  testWidgets('search matches descriptions and folds accents', (tester) async {
    await pumpSidebar(tester, active: SettingsSection.ui, onSelect: (_) {});
    final en = lookupAppLocalizations(const Locale('en'));
    final field = find.byType(MixarInput);

    await tester.enterText(field, 'midi');
    await tester.pump();
    expect(find.text(SettingsSection.controllers.label(en)), findsOneWidget);
    expect(find.text(SettingsSection.audio.label(en)), findsNothing);

    final pt = lookupAppLocalizations(const Locale('pt', 'BR'));
    await pumpSidebar(
      tester,
      active: SettingsSection.ui,
      onSelect: (_) {},
      locale: const Locale('pt', 'BR'),
    );
    await tester.enterText(find.byType(MixarInput), 'audio');
    await tester.pump();
    expect(find.text(SettingsSection.audio.label(pt)), findsOneWidget);

    await tester.enterText(find.byType(MixarInput), 'sessao');
    await tester.pump();
    expect(find.text(SettingsSection.session.label(pt)), findsOneWidget);
    expect(find.text(SettingsSection.audio.label(pt)), findsNothing);
  });

  testWidgets('shows a no-results message', (tester) async {
    SettingsSection? selected;
    await pumpSidebar(
      tester,
      active: SettingsSection.ui,
      onSelect: (section) => selected = section,
    );
    final l10n = lookupAppLocalizations(const Locale('en'));

    await tester.enterText(find.byType(MixarInput), 'zzz');
    await tester.pump();
    expect(selected, isNull);
    expect(find.text(l10n.settingsSearchNoResults), findsOneWidget);
    for (final section in kSettingsSections) {
      expect(find.text(section.label(l10n)), findsNothing);
    }

    await tester.tap(find.byIcon(LucideIcons.x));
    await tester.pump();
    expect(find.text(l10n.settingsSearchNoResults), findsNothing);
    expect(find.text(SettingsSection.ui.label(l10n)), findsOneWidget);
  });

  testWidgets('search field stays pinned while the nav list scrolls', (
    tester,
  ) async {
    final theme = MixarThemeData.dark();
    await tester.pumpWidget(
      mixarTestMaterialApp(
        theme: theme,
        home: Scaffold(
          body: SizedBox(
            height: 200,
            child: SettingsSidebar(
              active: SettingsSection.ui,
              onSelect: (_) {},
              onQueryChanged: (_) {},
            ),
          ),
        ),
      ),
    );

    final field = find.byType(MixarInput);
    final before = tester.getRect(field);
    final l10n = lookupAppLocalizations(const Locale('en'));
    final navList = find.ancestor(
      of: find.text(SettingsSection.ui.label(l10n)),
      matching: find.byType(SingleChildScrollView),
    );
    await tester.drag(navList, const Offset(0, -100));
    await tester.pump();
    expect(tester.getRect(field), before);
  });

  testWidgets('selected row tints only its icon with the accent', (
    tester,
  ) async {
    final theme = MixarThemeData.dark();
    await pumpSidebar(tester, active: SettingsSection.ui, onSelect: (_) {});

    final rows = find.byType(MTappable);
    final icons = <Icon>[
      for (var i = 0; i < kSettingsSections.length; i++)
        tester.widget<Icon>(
          find.descendant(of: rows.at(i), matching: find.byType(Icon)),
        ),
    ];
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
