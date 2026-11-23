import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/l10n/app_localizations.dart';
import 'package:gui_flutter/providers/settings/settings_providers.dart';
import 'package:gui_flutter/screens/settings_page.dart';
import 'package:gui_flutter/src/rust/api/engine.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';
import 'package:gui_flutter/widgets/settings/settings_section_card.dart';
import 'package:gui_flutter/widgets/settings/settings_section_sticky_header.dart';
import 'package:gui_flutter/widgets/settings/settings_ui_panel.dart';
import 'package:gui_flutter/widgets/shell/app_button.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:material_ui/material_ui.dart';

import 'support/mixar_material_app.dart';

void main() {
  final l10n = lookupAppLocalizations(const Locale('en'));

  /// Pumps the page in a window short enough that the section scrolls.
  Future<void> pumpSettings(WidgetTester tester) async {
    await tester.binding.setSurfaceSize(const Size(1100, 480));
    addTearDown(() => tester.binding.setSurfaceSize(null));
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          appSettingsProvider.overrideWith((ref) async => defaultAppSettings()),
          // Parity with the sibling settings tests: the page does not read the
          // device list itself, but a section that did would otherwise reach
          // the uninitialized bridge.
          audioDevicesProvider.overrideWith(
            (ref) async => const <OutputDevice>[],
          ),
        ],
        child: mixarTestMaterialApp(
          theme: MixarThemeData.dark(),
          home: const Scaffold(body: SettingsPage()),
        ),
      ),
    );
    await tester.pumpAndSettle();
  }

  /// Scopes a finder to the pinned section header.
  Finder inHeader(Finder matching) => find.descendant(
    of: find.byType(SettingsSectionStickyHeaderBody),
    matching: matching,
  );

  Future<void> scrollToEnd(WidgetTester tester) async {
    final scrollable = find.descendant(
      of: find.byKey(kSettingsContentScrollKey),
      matching: find.byType(Scrollable),
    );
    final position = tester.state<ScrollableState>(scrollable.first).position;
    expect(
      position.maxScrollExtent,
      greaterThan(0),
      reason: 'the panel must be scrollable for the header to compact',
    );
    position.jumpTo(position.maxScrollExtent);
    await tester.pumpAndSettle();
  }

  double headerTitleSize(WidgetTester tester) => tester
      .widget<Text>(inHeader(find.text(l10n.settingsSectionUi)))
      .style!
      .fontSize!;

  double headerDescriptionOpacity(WidgetTester tester) => tester
      .widget<Opacity>(
        find
            .ancestor(
              of: inHeader(find.text(l10n.settingsUiDescription)),
              matching: find.byType(Opacity),
            )
            .first,
      )
      .opacity;

  testWidgets('the section header pins and compacts as it scrolls', (
    tester,
  ) async {
    await pumpSettings(tester);

    final pane = tester.getRect(find.byKey(kSettingsContentScrollKey));
    final sliver = tester.widget<SliverPersistentHeader>(
      find.byType(SliverPersistentHeader),
    );
    // At rest the header holds a title row and the description; compact is the
    // title row alone.
    expect(sliver.delegate.minExtent, moreOrLessEquals(44));
    expect(sliver.delegate.maxExtent, greaterThan(90));

    const titleSize = 16.0;
    expect(headerTitleSize(tester), titleSize);
    expect(headerDescriptionOpacity(tester), 1);
    final restTitleTop = tester
        .getRect(inHeader(find.text(l10n.settingsSectionUi)))
        .top;
    expect(restTitleTop - pane.top, lessThan(40));

    await scrollToEnd(tester);

    // The header is still on screen — pinned — but as a single row: the
    // title shrank, the description faded out, and its row sits against the
    // pane top.
    expect(headerTitleSize(tester), lessThan(titleSize));
    expect(headerDescriptionOpacity(tester), 0);
    final compactTitleTop = tester
        .getRect(inHeader(find.text(l10n.settingsSectionUi)))
        .top;
    expect(compactTitleTop, greaterThanOrEqualTo(pane.top));
    expect(compactTitleTop, lessThan(restTitleTop));
    // Still inside the compact row at the pane top, not scrolled away.
    expect(compactTitleTop - pane.top, lessThan(20));
    // The cards scrolled under the header instead of carrying it away.
    expect(
      tester.getRect(find.byType(SettingsSectionCard).first).top,
      lessThan(pane.top + 44),
    );
  });

  testWidgets('the compact header keeps the Save button in view', (
    tester,
  ) async {
    await pumpSettings(tester);

    tester
        .widget<SettingsUiPanel>(find.byType(SettingsUiPanel))
        .onChanged(
          copyAppSettings(defaultAppSettings(), theme: ThemeSetting.light),
        );
    await tester.pumpAndSettle();

    await scrollToEnd(tester);

    final save = find.widgetWithText(AppButton, l10n.settingsSaveChanges);
    expect(save, findsOneWidget);
    final pane = tester.getRect(find.byKey(kSettingsContentScrollKey));
    expect(
      tester.getRect(save).top - pane.top,
      lessThan(44),
      reason: 'the Save button rides in the compact row',
    );
    expect(tester.getRect(save).bottom, greaterThan(pane.top));
  });
}
