import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/providers/settings/settings_providers.dart';
import 'package:gui_flutter/screens/settings_page.dart';
import 'package:gui_flutter/widgets/settings/mixxx_import_panel.dart';
import 'package:gui_flutter/widgets/settings/settings_library_panel.dart';
import 'package:gui_flutter/widgets/settings/settings_section_body.dart';
import 'package:gui_flutter/widgets/settings/settings_section_card.dart';
import 'package:gui_flutter/widgets/settings/settings_section_header.dart';
import 'package:gui_flutter/widgets/settings/settings_sidebar.dart';
import 'package:gui_flutter/widgets/settings/settings_ui_panel.dart';
import 'package:gui_flutter/widgets/shell/m_card.dart';
import 'package:gui_flutter/widgets/shell/m_divider.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:material_ui/material_ui.dart';

import 'support/mixar_material_app.dart';

void main() {
  Future<void> pumpLibrary(WidgetTester tester) async {
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
  }

  Future<void> pumpUi(WidgetTester tester, {bool meters = true}) async {
    await tester.pumpWidget(
      ProviderScope(
        child: mixarTestMaterialApp(
          theme: MixarThemeData.dark(),
          home: Scaffold(
            body: SingleChildScrollView(
              child: SettingsUiPanel(
                draft: copyAppSettings(
                  defaultAppSettings(),
                  showMixerMeters: meters,
                ),
                onChanged: (_) {},
              ),
            ),
          ),
        ),
      ),
    );
  }

  testWidgets('Library panel hosts list appearance controls', (tester) async {
    await pumpLibrary(tester);
    expect(find.text('Library list'), findsOneWidget);
    expect(find.text('Dim played tracks'), findsOneWidget);
    expect(find.text('TRACK ROW LAYOUT'), findsOneWidget);
    expect(
      find.widgetWithText(SettingsSectionCard, 'Visible fields'),
      findsOne,
    );
  }, semanticsEnabled: false);

  testWidgets('UI panel keeps chrome, meters, and drawer only', (tester) async {
    await pumpUi(tester);
    expect(find.text('Chrome and controls'), findsOneWidget);
    expect(find.text('Mixer meters'), findsOneWidget);
    expect(find.text('METER MODE'), findsOneWidget);
    expect(find.text('Library drawer'), findsOneWidget);
    expect(find.text('Library list'), findsNothing);
    expect(find.text('Dim played tracks'), findsNothing);
    expect(find.text('TRACK ROW LAYOUT'), findsNothing);
    expect(find.text('Analytics'), findsNothing);
  }, semanticsEnabled: false);

  testWidgets('UI panel hides meter mode when mixer meters off', (
    tester,
  ) async {
    await pumpUi(tester, meters: false);
    expect(find.text('Mixer meters'), findsOneWidget);
    expect(find.text('METER MODE'), findsNothing);
  }, semanticsEnabled: false);

  testWidgets('section cards are full-bleed bands with top/bottom hairlines', (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1200, 1400);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);
    await pumpLibrary(tester);

    final cards = find.byType(SettingsSectionCard);
    final rects = [
      for (var i = 0; i < cards.evaluate().length; i++)
        tester.getRect(cards.at(i)),
    ];
    expect(rects.length, greaterThan(1));
    for (final rect in rects) {
      expect(rect.left, 0);
      expect(rect.width, 1200);
    }
    for (var i = 1; i < rects.length; i++) {
      // A 16px gap separates the bands.
      expect(rects[i].top - rects[i - 1].bottom, 16);
    }
    // Each card draws a top and bottom hairline, no side chrome, no dividers.
    final cap = tester.widget<DecoratedBox>(
      find.descendant(
        of: cards.first,
        matching: find.byWidgetPredicate(
          (widget) =>
              widget is DecoratedBox &&
              widget.position == DecorationPosition.foreground,
        ),
      ),
    );
    final border = (cap.decoration as BoxDecoration).border! as Border;
    expect(border.top.width, 1);
    expect(border.bottom.width, 1);
    expect(border.left.width, 0);
    expect(border.right.width, 0);
    expect(
      find.descendant(
        of: find.byType(SettingsLibraryPanel),
        matching: find.byType(MDivider),
      ),
      findsNothing,
      reason: 'gaps separate the cards, not dividers',
    );
    expect(
      find.descendant(of: cards.first, matching: find.byType(MCard)),
      findsNothing,
      reason: 'settings cards are borderless on the sides',
    );
  }, semanticsEnabled: false);

  testWidgets('settings page cards bleed to the pane edges', (tester) async {
    tester.view.physicalSize = const Size(1600, 900);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);

    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          appSettingsProvider.overrideWith((ref) async => defaultAppSettings()),
        ],
        child: mixarTestMaterialApp(
          theme: MixarThemeData.dark(),
          home: const Scaffold(body: SettingsPage()),
        ),
      ),
    );
    await tester.pumpAndSettle();

    final card = tester.getRect(find.byType(SettingsSectionCard).first);
    final sidebar = tester.getRect(find.byType(SettingsSidebar));
    // No horizontal page margin: flush with the sidebar and the window edge.
    expect(card.right, 1600);
    expect(card.left, sidebar.right);

    // The section title/subtitle sit centred in the band above the first card.
    final scroll = tester.getRect(find.byType(SingleChildScrollView));
    final header = tester.getRect(find.byType(SettingsSectionHeader).first);
    expect(header.top - scroll.top, card.top - header.bottom);
  }, semanticsEnabled: false);

  testWidgets('section body renders the before slot above the cards', (
    tester,
  ) async {
    await tester.pumpWidget(
      mixarTestMaterialApp(
        theme: MixarThemeData.dark(),
        home: const Scaffold(
          body: SettingsSectionBody(
            title: 'Storage',
            description: 'Disk use.',
            before: Text('usage unavailable'),
            cards: [
              SettingsSectionCard(title: 'Overview'),
              SettingsSectionCard(title: 'Cleanup'),
            ],
          ),
        ),
      ),
    );

    final header = tester.getRect(find.byType(SettingsSectionHeader).first);
    final error = tester.getRect(find.text('usage unavailable'));
    final cards = find.byType(SettingsSectionCard);
    expect(header.bottom, lessThan(error.top));
    expect(error.bottom, lessThan(tester.getRect(cards.first).top));
    // The error does not break the card rhythm.
    expect(
      tester.getRect(cards.at(1)).top - tester.getRect(cards.at(0)).bottom,
      16,
    );
  }, semanticsEnabled: false);
}
