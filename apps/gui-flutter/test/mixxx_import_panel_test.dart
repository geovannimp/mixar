import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/settings/mixxx_import_panel.dart';
import 'package:gui_flutter/shell/app_button.dart';
import 'package:gui_flutter/shell/material_theme.dart';
import 'package:gui_flutter/shell/mixar_theme.dart';
import 'package:gui_flutter/src/rust/api/library.dart';
import 'package:material_ui/material_ui.dart';

import 'support/mixar_material_app.dart';

Future<void> _pump(
  WidgetTester tester, {
  required Future<String?> Function() path,
}) async {
  final theme = MixarThemeData.dark();
  await tester.pumpWidget(
    ProviderScope(
      overrides: [mixxxDatabasePathProvider.overrideWith((ref) => path())],
      child: MaterialApp(
        theme: materialUiThemeFromMixar(theme),
        builder: mixarMaterialAppBuilder(theme),
        home: const Scaffold(body: MixxxImportPanel()),
      ),
    ),
  );
  await tester.pumpAndSettle();
}

void main() {
  testWidgets('disables the action when no Mixxx database is found', (
    tester,
  ) async {
    await _pump(tester, path: () async => null);
    expect(find.text('Import from Mixxx'), findsOneWidget);
    expect(
      find.text('No Mixxx library found on this computer.'),
      findsOneWidget,
    );
    expect(tester.widget<AppButton>(find.byType(AppButton)).onPress, isNull);
  }, semanticsEnabled: false);

  testWidgets('enables the action when a Mixxx database is found', (
    tester,
  ) async {
    await _pump(tester, path: () async => '/home/me/.mixxx/mixxxdb.sqlite');
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
    expect(find.text('Could not check for a Mixxx library.'), findsOneWidget);
    expect(tester.widget<AppButton>(find.byType(AppButton)).onPress, isNull);
  }, semanticsEnabled: false);

  test('summary lists added, updated, and collections', () {
    const report = MixxxImportReport(
      tracksAdded: 3,
      tracksUpdated: 2,
      tracksMissingFiles: 1,
      foldersImported: 1,
      playlistsImported: 1,
      cratesImported: 0,
      collectionsSkipped: 0,
      failed: 0,
      errors: [],
    );
    final summary = mixxxImportSummary(report);
    expect(summary, contains('3 tracks'));
    expect(summary, contains('2 updated'));
    expect(summary, contains('1 playlist'));
    expect(summary, contains('1 folder'));
    expect(summary, contains('1 missing'));
  });

  test('summary reports already-imported when only updates/skips', () {
    const skipped = MixxxImportReport(
      tracksAdded: 0,
      tracksUpdated: 0,
      tracksMissingFiles: 0,
      foldersImported: 0,
      playlistsImported: 0,
      cratesImported: 0,
      collectionsSkipped: 2,
      failed: 0,
      errors: [],
    );
    expect(mixxxImportSummary(skipped), 'Mixxx library already imported');

    const updated = MixxxImportReport(
      tracksAdded: 0,
      tracksUpdated: 5,
      tracksMissingFiles: 0,
      foldersImported: 0,
      playlistsImported: 0,
      cratesImported: 0,
      collectionsSkipped: 0,
      failed: 0,
      errors: [],
    );
    expect(mixxxImportSummary(updated), 'Updated 5 tracks from Mixxx');
  });
}
