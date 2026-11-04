import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/l10n/app_localizations.dart';
import 'package:gui_flutter/src/rust/api/library.dart';
import 'package:gui_flutter/widgets/settings/rekordbox_import_panel.dart';
import 'package:gui_flutter/widgets/shell/app_button.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:material_ui/material_ui.dart';

import 'support/mixar_material_app.dart';

Future<void> _pump(
  WidgetTester tester, {
  required Future<String?> Function() path,
}) async {
  final theme = MixarThemeData.dark();
  await tester.pumpWidget(
    ProviderScope(
      overrides: [rekordboxDatabasePathProvider.overrideWith((ref) => path())],
      child: mixarTestMaterialApp(
        theme: theme,
        home: const Scaffold(body: RekordboxImportPanel()),
      ),
    ),
  );
  await tester.pumpAndSettle();
}

void main() {
  final l10n = lookupAppLocalizations(const Locale('en'));

  testWidgets('disables the action when no Rekordbox database is found', (
    tester,
  ) async {
    await _pump(tester, path: () async => null);
    expect(find.text(l10n.rekordboxImportTitle), findsOneWidget);
    expect(find.text(l10n.rekordboxImportNotFound), findsOneWidget);
    expect(tester.widget<AppButton>(find.byType(AppButton)).onPress, isNull);
    expect(
      tester.getTopLeft(find.text(l10n.rekordboxImportNotFound)).dx,
      tester.getTopLeft(find.text(l10n.rekordboxImportTitle)).dx,
      reason: 'the status line sits under the title, not centered in the card',
    );
  }, semanticsEnabled: false);

  testWidgets('enables the action when a Rekordbox database is found', (
    tester,
  ) async {
    await _pump(
      tester,
      path: () async => '/home/me/Library/Pioneer/rekordbox/master.db',
    );
    expect(find.textContaining('Found:'), findsOneWidget);
    expect(tester.widget<AppButton>(find.byType(AppButton)).onPress, isNotNull);
  }, semanticsEnabled: false);

  testWidgets('reports a probe failure distinctly and keeps the action off', (
    tester,
  ) async {
    await _pump(
      tester,
      path: () async => throw StateError('bridge unavailable'),
    );
    expect(find.text(l10n.rekordboxImportCheckFailed), findsOneWidget);
    expect(tester.widget<AppButton>(find.byType(AppButton)).onPress, isNull);
  }, semanticsEnabled: false);

  test('summary lists added tracks, playlists, cues, and loops', () {
    const report = RekordboxImportReport(
      tracksAdded: 3,
      tracksUpdated: 2,
      tracksMissingFiles: 1,
      playlistsImported: 1,
      collectionsSkipped: 0,
      hotCuesImported: 4,
      loopsImported: 2,
      failed: 0,
      errors: [],
    );
    final summary = rekordboxImportSummary(l10n, report);
    expect(summary, contains('3 tracks'));
    expect(summary, contains('2 updated'));
    expect(summary, contains('1 playlist'));
    expect(summary, contains('4 hot cues'));
    expect(summary, contains('2 loops'));
    expect(summary, contains('1 missing'));
  });

  test('summary reports already-imported when only updates/skips', () {
    const skipped = RekordboxImportReport(
      tracksAdded: 0,
      tracksUpdated: 0,
      tracksMissingFiles: 0,
      playlistsImported: 0,
      collectionsSkipped: 2,
      hotCuesImported: 0,
      loopsImported: 0,
      failed: 0,
      errors: [],
    );
    expect(
      rekordboxImportSummary(l10n, skipped),
      l10n.rekordboxImportAlreadyImported,
    );

    const updated = RekordboxImportReport(
      tracksAdded: 0,
      tracksUpdated: 5,
      tracksMissingFiles: 0,
      playlistsImported: 0,
      collectionsSkipped: 0,
      hotCuesImported: 0,
      loopsImported: 0,
      failed: 0,
      errors: [],
    );
    expect(
      rekordboxImportSummary(l10n, updated),
      l10n.rekordboxImportUpdated(l10n.rekordboxCountUpdated(5), ''),
    );
  });

  test('a cue-only re-import still reports the copied cues', () {
    const cuesOnly = RekordboxImportReport(
      tracksAdded: 0,
      tracksUpdated: 0,
      tracksMissingFiles: 0,
      playlistsImported: 0,
      collectionsSkipped: 0,
      hotCuesImported: 2,
      loopsImported: 1,
      failed: 0,
      errors: [],
    );
    final summary = rekordboxImportSummary(l10n, cuesOnly);
    expect(summary, contains('2 hot cues'));
    expect(summary, contains('1 loop'));
  });
}
