import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
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
import 'package:gui_flutter/widgets/shell/mixar_input.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';
import 'package:material_ui/material_ui.dart';

import 'support/mixar_material_app.dart';

void main() {
  Future<void> pumpSettings(WidgetTester tester) async {
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
          home: const Scaffold(body: SettingsPage()),
        ),
      ),
    );
    await tester.pumpAndSettle();
  }

  Future<void> search(WidgetTester tester, String query) async {
    await tester.enterText(
      find.descendant(
        of: find.byType(SettingsSidebar),
        matching: find.byType(MixarInput),
      ),
      query,
    );
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

    await tester.tap(find.byIcon(LucideIcons.x));
    await tester.pumpAndSettle();
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
