import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/models/settings_section.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/core/utils/settings_search.dart';
import 'package:gui_flutter/l10n/app_localizations.dart';
import 'package:gui_flutter/providers/settings/settings_providers.dart';
import 'package:gui_flutter/screens/settings_page.dart';
import 'package:gui_flutter/widgets/settings/mixxx_import_panel.dart';
import 'package:gui_flutter/widgets/settings/rekordbox_import_panel.dart';
import 'package:gui_flutter/widgets/settings/settings_search_scope.dart';
import 'package:gui_flutter/widgets/settings/settings_section_body.dart';
import 'package:gui_flutter/widgets/settings/settings_section_card.dart';
import 'package:gui_flutter/widgets/settings/settings_sidebar.dart';
import 'package:gui_flutter/widgets/settings/settings_ui_panel.dart';
import 'package:gui_flutter/widgets/shell/app_button.dart';
import 'package:gui_flutter/widgets/shell/mixar_input.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';
import 'package:material_ui/material_ui.dart';

import 'support/mixar_material_app.dart';

void main() {
  Future<void> pumpSettings(
    WidgetTester tester, {
    SettingsSection initialSection = SettingsSection.ui,
  }) async {
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          appSettingsProvider.overrideWith(
            (ref) async => copyAppSettings(defaultAppSettings(), numDecks: 2),
          ),
          mixxxDatabasePathProvider.overrideWith((ref) async => null),
          rekordboxDatabasePathProvider.overrideWith((ref) async => null),
        ],
        child: mixarTestMaterialApp(
          theme: MixarThemeData.dark(),
          home: Scaffold(body: SettingsPage(initialSection: initialSection)),
        ),
      ),
    );
    await tester.pumpAndSettle();
  }

  /// Filled searches close from the right-edge clear button.
  Future<void> closeSearch(WidgetTester tester, Locale locale) async {
    await tester.tap(
      find.bySemanticsLabel(lookupAppLocalizations(locale).settingsSearchClose),
    );
    await tester.pumpAndSettle();
  }

  /// A press outside the header's field closes the search bar. The corner
  /// cell is the safest target: it holds the marker, so nothing else reacts.
  Future<void> closeByTappingOutside(WidgetTester tester) async {
    final page = tester.getRect(find.byType(SettingsPage));
    await tester.tapAt(
      Offset(
        page.left + kSettingsHeaderHeight / 2,
        page.top + kSettingsHeaderHeight / 2,
      ),
    );
  }

  /// The search lives in the header: press the button, then type.
  Future<void> search(WidgetTester tester, String query) async {
    await tester.tap(find.byIcon(LucideIcons.search));
    await tester.pumpAndSettle();
    await tester.enterText(find.byType(MixarInput), query);
    await tester.pumpAndSettle();
  }

  testWidgets('search narrows the active panel to matching cards', (
    tester,
  ) async {
    await pumpSettings(tester);
    final l10n = lookupAppLocalizations(const Locale('en'));

    await search(tester, 'theme');

    expect(find.text(l10n.settingsUiThemeTitle), findsOneWidget);
    expect(find.text(l10n.settingsUiChromeTitle), findsNothing);
    expect(find.text(l10n.settingsUiOnboardingTitle), findsNothing);
  });

  testWidgets('falls back to the whole section when no card matches', (
    tester,
  ) async {
    await pumpSettings(tester);
    final l10n = lookupAppLocalizations(const Locale('en'));

    await search(tester, 'audio');

    expect(find.text(l10n.settingsAudioEngineTitle), findsOneWidget);
    expect(find.text(l10n.settingsAudioMasterBus), findsOneWidget);
    expect(find.text(l10n.settingsAudioPreviewBusTitle), findsOneWidget);
  });

  testWidgets('shows a no-results message with the query', (tester) async {
    await pumpSettings(tester);
    final l10n = lookupAppLocalizations(const Locale('en'));

    await search(tester, 'zzzz');

    expect(find.text(l10n.settingsSearchNoResultsFor('zzzz')), findsOneWidget);
    // The previously selected section's content is replaced, not kept.
    expect(find.text(l10n.settingsUiThemeTitle), findsNothing);
    expect(find.text(l10n.settingsUiChromeTitle), findsNothing);

    await closeSearch(tester, const Locale('en'));
    expect(find.text(l10n.settingsSearchNoResultsFor('zzzz')), findsNothing);
    expect(find.text(l10n.settingsUiThemeTitle), findsOneWidget);
  });

  testWidgets('finds import cards by product name', (tester) async {
    await pumpSettings(tester);
    final l10n = lookupAppLocalizations(const Locale('en'));

    await search(tester, 'Rekordbox');

    expect(
      find.descendant(
        of: find.byType(SettingsSidebar),
        matching: find.text(l10n.settingsSectionIntegrations),
      ),
      findsOneWidget,
    );
    expect(find.text(l10n.rekordboxImportTitle), findsOneWidget);
    expect(find.text(l10n.mixxxImportTitle), findsNothing);
    expect(find.text(l10n.curvesImportExportTitle), findsNothing);
  });

  testWidgets('finds cards by their subtitle terms', (tester) async {
    await pumpSettings(tester);
    final l10n = lookupAppLocalizations(const Locale('en'));

    await search(tester, 'buffer');

    expect(find.text(l10n.settingsAudioEngineTitle), findsOneWidget);
    expect(find.text(l10n.settingsAudioMasterBus), findsNothing);
    expect(find.text(l10n.settingsAudioPreviewBusTitle), findsNothing);
  });

  testWidgets('finds cards by subtitle-only terms', (tester) async {
    await pumpSettings(tester);
    final l10n = lookupAppLocalizations(const Locale('en'));

    await search(tester, 'cue');

    expect(find.text(l10n.settingsAudioPreviewBusTitle), findsOneWidget);
    expect(find.text(l10n.settingsAudioEngineTitle), findsNothing);
    expect(find.text(l10n.settingsAudioMasterBus), findsNothing);
  });

  testWidgets('field-label terms keep the owning card in the panel', (
    tester,
  ) async {
    await tester.pumpWidget(
      mixarTestMaterialApp(
        theme: MixarThemeData.dark(),
        home: const Scaffold(
          body: SettingsSearchScope(
            query: 'fader',
            child: SettingsSectionBody(
              title: 'Mixer',
              description: '',
              cards: [
                SettingsSectionCard(title: 'Fader curve', child: Text('curve')),
                SettingsSectionCard(
                  title: 'Channel meters',
                  searchTerms: ['fader taper'],
                  child: Text('meters'),
                ),
                SettingsSectionCard(title: 'Output', child: Text('out')),
              ],
            ),
          ),
        ),
      ),
    );

    // 'fader' hits one card by title and one by a field label it owns; the
    // label-matched card must survive the filter rather than losing to the
    // title match.
    expect(find.text('Fader curve'), findsOneWidget);
    expect(find.text('Channel meters'), findsOneWidget);
    expect(find.text('Output'), findsNothing);
  });

  testWidgets('the header search button expands the header into a field', (
    tester,
  ) async {
    await pumpSettings(tester);
    final l10n = lookupAppLocalizations(const Locale('en'));

    expect(find.byType(MixarInput), findsNothing);
    final searchButton = find.bySemanticsLabel(l10n.settingsSearchHint);
    expect(searchButton, findsOneWidget);
    final header = tester.getRect(find.bySemanticsLabel(l10n.settingsTitle));

    await tester.tap(searchButton);
    await tester.pumpAndSettle();

    final field = find.byType(MixarInput);
    expect(field, findsOneWidget);
    expect(tester.widget<MixarInput>(field).focusNode?.hasFocus, isTrue);
    // The field takes the title's room inside the header band.
    expect(tester.getRect(field).top, lessThan(header.bottom));
    expect(find.text(l10n.settingsTitle), findsNothing);
    expect(find.text(l10n.settingsSubtitle), findsNothing);
  });

  testWidgets('the open search is a marker, closed by a press outside', (
    tester,
  ) async {
    await pumpSettings(tester);
    await tester.tap(find.widgetWithIcon(AppButton, LucideIcons.search));
    await tester.pumpAndSettle();
    expect(find.byType(MixarInput), findsOneWidget);

    // No search control left: the square that moved is the page's marker now,
    // the way the settings glyph was.
    expect(
      find.widgetWithIcon(AppButton, LucideIcons.search),
      findsNothing,
      reason: 'the moved square is a marker, not a control',
    );

    // The clear button holds the vacated edge seat.
    expect(find.widgetWithIcon(AppButton, LucideIcons.x), findsOneWidget);

    await closeByTappingOutside(tester);
    await tester.pumpAndSettle();
    expect(find.byType(MixarInput), findsNothing);
    expect(
      find.descendant(
        of: find.byType(SettingsPage),
        matching: find.widgetWithIcon(AppButton, LucideIcons.search),
      ),
      findsOneWidget,
    );
  });

  testWidgets('the save slot follows the ambient text scale', (tester) async {
    // A scaled-up label is wider than raw font metrics; the slot must grow with
    // it instead of clipping the button (a real overflow on a scaled desktop).
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          appSettingsProvider.overrideWith(
            (ref) async => copyAppSettings(defaultAppSettings(), numDecks: 2),
          ),
        ],
        child: mixarTestMaterialApp(
          theme: MixarThemeData.dark(),
          home: const MediaQuery(
            data: MediaQueryData(textScaler: TextScaler.linear(1.4)),
            child: Scaffold(body: SettingsPage()),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    tester
        .widget<SettingsUiPanel>(find.byType(SettingsUiPanel))
        .onChanged(copyAppSettings(defaultAppSettings(), showTooltips: false));
    await tester.pumpAndSettle();

    expect(tester.takeException(), isNull);
    expect(find.widgetWithText(AppButton, 'Save changes'), findsOneWidget);
  });

  testWidgets('a press outside keeps a filled search open', (tester) async {
    await pumpSettings(tester);
    await tester.tap(find.widgetWithIcon(AppButton, LucideIcons.search));
    await tester.pumpAndSettle();
    await tester.enterText(find.byType(MixarInput), 'theme');
    await tester.pumpAndSettle();

    await closeByTappingOutside(tester);
    await tester.pumpAndSettle();

    expect(find.byType(MixarInput), findsOneWidget);
    expect(
      tester.widget<MixarInput>(find.byType(MixarInput)).controller?.text,
      'theme',
    );
    // The right-edge clear button is the way out while there is a query.
    expect(find.widgetWithIcon(AppButton, LucideIcons.x), findsOneWidget);
  });

  testWidgets('the search button slides from the header edge into the corner', (
    tester,
  ) async {
    await pumpSettings(tester);
    // The glyph, not the button: the square stops being a control once it
    // arrives, and the same glyph rides along either way.
    final glyph = find.byIcon(LucideIcons.search);
    final toggle = find.widgetWithIcon(AppButton, LucideIcons.search);
    final closed = tester.getRect(glyph);

    await tester.tap(toggle);
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 100));
    final midFlight = tester.getRect(glyph);
    await tester.pumpAndSettle();
    final open = tester.getRect(glyph);

    // It travels left rather than jumping: both seats and the frame between
    // them are distinct.
    expect(open.left, lessThan(closed.left));
    expect(midFlight.left, lessThan(closed.left));
    expect(midFlight.left, greaterThan(open.left));
    // The corner is the square cell's own seat, and the glyph lands centred in
    // it.
    expect(open.left, greaterThanOrEqualTo(kSettingsHeaderHeight / 2 - 8));
    expect(open.right, lessThanOrEqualTo(kSettingsHeaderHeight));
    expect(find.byIcon(LucideIcons.settings), findsOneWidget);
  });

  testWidgets('a dirtied draft opens the Save slot and shifts the controls', (
    tester,
  ) async {
    await pumpSettings(tester);
    final toggle = find.widgetWithIcon(AppButton, LucideIcons.search);
    final closed = tester.getRect(toggle);
    final headerRight = tester.getRect(find.byType(SettingsPage)).right;
    expect(find.widgetWithText(AppButton, 'Save changes'), findsNothing);

    tester
        .widget<SettingsUiPanel>(find.byType(SettingsUiPanel))
        .onChanged(copyAppSettings(defaultAppSettings(), showTooltips: false));
    await tester.pumpAndSettle();

    final save = tester.getRect(find.widgetWithText(AppButton, 'Save changes'));
    final shifted = tester.getRect(toggle);
    expect(
      shifted,
      closed,
      reason: 'the square keeps the header edge; the slot opens beside it',
    );
    expect(save.right, moreOrLessEquals(closed.left - 8));
    expect(closed.right, moreOrLessEquals(headerRight));
  });

  testWidgets('closing the search goes back to the section it hid', (
    tester,
  ) async {
    await pumpSettings(tester, initialSection: SettingsSection.audio);
    final l10n = lookupAppLocalizations(const Locale('en'));

    // Audio is hidden by 'theme', so the page moves to UI to keep the panel in
    // sync with the filtered nav.
    await search(tester, 'theme');
    expect(find.text(l10n.settingsUiThemeTitle), findsOneWidget);

    await closeSearch(tester, const Locale('en'));

    expect(find.text(l10n.settingsAudioEngineTitle), findsOneWidget);
    expect(find.text(l10n.settingsUiThemeTitle), findsNothing);
    expect(find.byType(MixarInput), findsNothing);
  });

  testWidgets('a row picked while searching survives the clear', (
    tester,
  ) async {
    await pumpSettings(tester, initialSection: SettingsSection.audio);
    final l10n = lookupAppLocalizations(const Locale('en'));

    await search(tester, 'theme');
    // A press outside keeps a filled search open, and the row still lands.
    await tester.tap(
      find.descendant(
        of: find.byType(SettingsSidebar),
        matching: find.text(l10n.settingsSectionUi),
      ),
    );
    await tester.pumpAndSettle();
    expect(find.byType(MixarInput), findsOneWidget);

    await closeSearch(tester, const Locale('en'));

    expect(find.text(l10n.settingsUiThemeTitle), findsOneWidget);
    expect(find.text(l10n.settingsAudioEngineTitle), findsNothing);
  });

  testWidgets('finds sections despite typos', (tester) async {
    await pumpSettings(tester);
    final l10n = lookupAppLocalizations(const Locale('en'));

    await search(tester, 'rekordbxo');

    expect(
      find.descendant(
        of: find.byType(SettingsSidebar),
        matching: find.text(l10n.settingsSectionIntegrations),
      ),
      findsOneWidget,
    );
  });

  group('settingsSearchFuzzyScore', () {
    test('identical text scores 100', () {
      expect(settingsSearchFuzzyScore('rekordbox', ['Rekordbox']), 100);
    });

    test('queries under three characters are ignored', () {
      expect(settingsSearchFuzzyScore('re', ['Rekordbox']), 0);
    });

    test('a transposed typo clears the threshold', () {
      expect(
        settingsSearchFuzzyScore('rekordbxo', ['Rekordbox']),
        greaterThanOrEqualTo(settingsSearchFuzzyThreshold),
      );
    });

    test('unrelated text stays below the threshold', () {
      expect(
        settingsSearchFuzzyScore('zzzz', ['Rekordbox']),
        lessThan(settingsSearchFuzzyThreshold),
      );
    });

    test('empty candidates score zero', () {
      expect(settingsSearchFuzzyScore('rekordbox', const []), 0);
    });
  });
}
