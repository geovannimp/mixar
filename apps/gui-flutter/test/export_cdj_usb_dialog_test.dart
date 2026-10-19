import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/l10n/app_localizations.dart';
import 'package:gui_flutter/providers/library/providers.dart';
import 'package:gui_flutter/src/rust/api/library.dart';
import 'package:gui_flutter/widgets/library/collection_actions_menu.dart';
import 'package:gui_flutter/widgets/library/export_cdj_usb_dialog.dart';
import 'package:gui_flutter/widgets/settings/settings_field.dart';
import 'package:gui_flutter/widgets/shell/app_button.dart';
import 'package:gui_flutter/widgets/shell/mixar_checkbox.dart';
import 'package:gui_flutter/widgets/shell/mixar_input.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';
import 'package:material_ui/material_ui.dart';

import 'support/fake_library_transport.dart';
import 'support/mixar_material_app.dart';

const _playlist = LibraryCollectionSummary(
  id: 'pl-warmup',
  name: 'Warmup',
  kind: 'playlist',
  trackCount: 3,
);

const _folder = LibraryCollectionSummary(
  id: 'fo-sets',
  name: 'Sets',
  kind: 'folder',
  path: '/music/sets',
  trackCount: 2,
);

/// Directory picker double: records the dialog titles it was asked with.
class _FakeDirectoryPicker {
  new({this.directory});

  /// Returned path, or `null` for a cancelled pick.
  final String? directory;

  final List<String?> dialogTitles = [];

  Future<String?> call(String? dialogTitle) async {
    dialogTitles.add(dialogTitle);
    return directory;
  }
}

CdjExportReport _report({
  int tracks = 3,
  int tracksReused = 1,
  int playlists = 1,
  int folders = 0,
  int bytesCopied = 2048,
  List<String> errors = const [],
  List<String> warnings = const [],
}) => CdjExportReport(
  tracks: tracks,
  tracksReused: tracksReused,
  playlists: playlists,
  folders: folders,
  bytesCopied: BigInt.from(bytesCopied),
  errors: errors,
  warnings: warnings,
);

/// Mount an app that opens the export dialog for [collection] on first tap.
Future<void> _pump(
  WidgetTester tester, {
  required FakeLibraryTransport transport,
  LibraryCollectionSummary collection = _playlist,
  _FakeDirectoryPicker? picker,
}) async {
  tester.view.physicalSize = const Size(1000, 1600);
  tester.view.devicePixelRatio = 1;
  addTearDown(tester.view.reset);
  await tester.pumpWidget(
    ProviderScope(
      overrides: [
        libraryTransportProvider.overrideWith((ref) async => transport),
        if (picker != null)
          cdjDirectoryPickerProvider.overrideWithValue(picker.call),
      ],
      child: mixarTestMaterialApp(
        theme: MixarThemeData.dark(),
        home: Scaffold(
          body: Builder(
            builder: (context) => AppButton(
              onPress: () =>
                  showExportCdjUsbDialog(context, collection: collection),
              child: const Text('open'),
            ),
          ),
        ),
      ),
    ),
  );
  await tester.pumpAndSettle();
  await tester.tap(find.text('open'));
  await tester.pumpAndSettle();
}

Future<void> _press(WidgetTester tester, Finder finder) async {
  await tester.ensureVisible(finder);
  await tester.pumpAndSettle();
  await tester.tap(finder);
  await tester.pumpAndSettle();
}

Finder _exportButton(AppLocalizations l10n) =>
    find.widgetWithText(AppButton, l10n.cdjExportConfirmAction);

void main() {
  final l10n = lookupAppLocalizations(const Locale('en'));

  _menuEntryPointTests(l10n);

  testWidgets('export stays disabled until a destination is set', (
    tester,
  ) async {
    final transport = FakeLibraryTransport();
    addTearDown(transport.dispose);
    await _pump(tester, transport: transport);

    expect(find.text(l10n.cdjExportTitle), findsOneWidget);
    expect(find.text('Warmup'), findsOneWidget);
    expect(tester.widget<AppButton>(_exportButton(l10n)).onPress, isNull);

    // The "choose a destination" hint belongs to the destination field itself
    // (directly under the input), not to the dialog body.
    final destinationField = find.ancestor(
      of: find.byType(MixarInput),
      matching: find.byType(SettingsField),
    );
    expect(destinationField, findsOneWidget);
    expect(
      find.descendant(
        of: destinationField,
        matching: find.text(l10n.cdjExportDestinationMissing),
      ),
      findsOneWidget,
    );

    await tester.enterText(find.byType(MixarInput), '/media/usb');
    await tester.pumpAndSettle();

    expect(tester.widget<AppButton>(_exportButton(l10n)).onPress, isNotNull);
    expect(transport.exportCdjUsbCalls, isEmpty);
  }, semanticsEnabled: false);

  testWidgets('Browse asks the platform directory picker', (tester) async {
    final picker = _FakeDirectoryPicker(directory: '/media/usb');
    final transport = FakeLibraryTransport(
      exportCdjUsbHandler: (_) async => _report(),
    );
    addTearDown(transport.dispose);
    await _pump(tester, transport: transport, picker: picker);

    await _press(tester, find.widgetWithText(AppButton, l10n.cdjExportBrowse));

    expect(picker.dialogTitles, [l10n.cdjExportTitle]);
    expect(find.text('/media/usb'), findsOneWidget);
    expect(tester.widget<AppButton>(_exportButton(l10n)).onPress, isNotNull);
  }, semanticsEnabled: false);

  testWidgets('a cancelled pick leaves the destination empty', (tester) async {
    final transport = FakeLibraryTransport();
    addTearDown(transport.dispose);
    await _pump(tester, transport: transport, picker: _FakeDirectoryPicker());

    await _press(tester, find.widgetWithText(AppButton, l10n.cdjExportBrowse));

    expect(find.text(l10n.cdjExportDestinationMissing), findsOneWidget);
    expect(tester.widget<AppButton>(_exportButton(l10n)).onPress, isNull);
  }, semanticsEnabled: false);

  testWidgets('cancelling the dialog does not export', (tester) async {
    final transport = FakeLibraryTransport(
      exportCdjUsbHandler: (_) async => _report(),
    );
    addTearDown(transport.dispose);
    await _pump(tester, transport: transport);
    await tester.enterText(find.byType(MixarInput), '/media/usb');
    await tester.pumpAndSettle();

    await _press(tester, find.text(l10n.settingsCancel));

    expect(transport.exportCdjUsbCalls, isEmpty);
    expect(find.text(l10n.cdjExportTitle), findsNothing);
  }, semanticsEnabled: false);

  testWidgets('a playlist export sends the playlist and toasts the report', (
    tester,
  ) async {
    final transport = FakeLibraryTransport(
      exportCdjUsbHandler: (_) async => _report(),
    );
    addTearDown(transport.dispose);
    await _pump(tester, transport: transport);
    await tester.enterText(find.byType(MixarInput), '/media/usb');
    await tester.pumpAndSettle();

    await _press(tester, _exportButton(l10n));

    expect(transport.exportCdjUsbCalls, hasLength(1));
    final request = transport.exportCdjUsbCalls.single;
    expect(request.destination, '/media/usb');
    expect(request.playlists, ['pl-warmup']);
    expect(request.folders, isEmpty);
    expect(request.refreshFiles, isFalse);
    expect(find.text(cdjExportSummary(l10n, _report())), findsOneWidget);
    // The dialog closes once the run is reported.
    expect(find.text(l10n.cdjExportTitle), findsNothing);
  }, semanticsEnabled: false);

  testWidgets('a folder export sends the folder collection', (tester) async {
    final transport = FakeLibraryTransport(
      exportCdjUsbHandler: (_) async => _report(folders: 1, bytesCopied: 4096),
    );
    addTearDown(transport.dispose);
    await _pump(tester, transport: transport, collection: _folder);
    await tester.enterText(find.byType(MixarInput), '/media/usb');
    await tester.pumpAndSettle();

    await _press(tester, _exportButton(l10n));

    final request = transport.exportCdjUsbCalls.single;
    expect(request.folders, ['fo-sets']);
    expect(request.playlists, isEmpty);
  }, semanticsEnabled: false);

  testWidgets('the Override checkbox reaches the request', (tester) async {
    final transport = FakeLibraryTransport(
      exportCdjUsbHandler: (_) async => _report(),
    );
    addTearDown(transport.dispose);
    await _pump(tester, transport: transport);
    await tester.enterText(find.byType(MixarInput), '/media/usb');
    await tester.pumpAndSettle();

    // A checkbox next to its label, with no explanatory hint line.
    expect(find.byType(MixarCheckbox), findsOneWidget);
    expect(
      find.descendant(
        of: find.byType(MixarCheckbox),
        matching: find.text(l10n.cdjExportOverride),
      ),
      findsOneWidget,
    );

    await _press(tester, find.text(l10n.cdjExportOverride));

    await _press(tester, _exportButton(l10n));

    expect(transport.exportCdjUsbCalls.single.refreshFiles, isTrue);
  }, semanticsEnabled: false);

  testWidgets('a failed export keeps the dialog open and toasts the error', (
    tester,
  ) async {
    final transport = FakeLibraryTransport(
      exportCdjUsbHandler: (_) async => throw StateError('drive unplugged'),
    );
    addTearDown(transport.dispose);
    await _pump(tester, transport: transport);
    await tester.enterText(find.byType(MixarInput), '/media/usb');
    await tester.pumpAndSettle();

    await _press(tester, _exportButton(l10n));

    expect(find.text(l10n.cdjExportFailed), findsOneWidget);
    expect(find.textContaining('drive unplugged'), findsOneWidget);
    // Still open, ready to retry.
    expect(find.text(l10n.cdjExportTitle), findsOneWidget);
    expect(tester.widget<AppButton>(_exportButton(l10n)).onPress, isNotNull);
  }, semanticsEnabled: false);

  testWidgets('warnings keep the counts next to the warning', (tester) async {
    final transport = FakeLibraryTransport(
      exportCdjUsbHandler: (_) async =>
          _report(warnings: ['drop.mp3: not analyzed']),
    );
    addTearDown(transport.dispose);
    await _pump(tester, transport: transport);
    await tester.enterText(find.byType(MixarInput), '/media/usb');
    await tester.pumpAndSettle();

    await _press(tester, _exportButton(l10n));

    final warned = _report(warnings: ['drop.mp3: not analyzed']);
    expect(find.text(l10n.cdjExportFinishedWithWarnings(1)), findsOneWidget);
    expect(
      find.text('drop.mp3: not analyzed\n${cdjExportSummary(l10n, warned)}'),
      findsOneWidget,
    );
  }, semanticsEnabled: false);

  test('summary lists tracks, reused files, collections, and bytes', () {
    final summary = cdjExportSummary(l10n, _report(folders: 1));
    expect(summary, contains('3 tracks'));
    expect(summary, contains('1 reused'));
    expect(summary, contains('1 playlist'));
    expect(summary, contains('1 folder'));
    expect(summary, contains('2.0 KB copied'));
  });

  test('summary reports nothing when the export wrote nothing', () {
    final summary = cdjExportSummary(
      l10n,
      _report(tracks: 0, tracksReused: 0, playlists: 0, bytesCopied: 0),
    );
    expect(summary, l10n.cdjExportNothing);
  });
}

/// The entry point itself: the collection's three-dot menu opens the dialog.
void _menuEntryPointTests(AppLocalizations l10n) {
  testWidgets('the collection menu opens the export dialog', (tester) async {
    final transport = FakeLibraryTransport(
      exportCdjUsbHandler: (_) async => _report(),
    );
    addTearDown(transport.dispose);
    tester.view.physicalSize = const Size(1000, 1600);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          libraryTransportProvider.overrideWith((ref) async => transport),
        ],
        child: mixarTestMaterialApp(
          theme: MixarThemeData.dark(),
          home: const Scaffold(
            body: Center(child: CollectionActionsMenu(collection: _playlist)),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    await _press(tester, find.byIcon(LucideIcons.ellipsisVertical));
    expect(find.text('Export to CDJ USB…'), findsOneWidget);

    await _press(tester, find.text('Export to CDJ USB…'));

    expect(find.text(l10n.cdjExportTitle), findsOneWidget);
    expect(find.text('Warmup'), findsOneWidget);
    expect(find.text(l10n.cdjExportDestinationMissing), findsOneWidget);
  }, semanticsEnabled: false);
}
