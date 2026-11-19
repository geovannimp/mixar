import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/theme/material_theme.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/providers/library/providers.dart';
import 'package:gui_flutter/providers/settings/settings_providers.dart';
import 'package:gui_flutter/src/rust/api/fs_browser.dart';
import 'package:gui_flutter/src/rust/api/library.dart';
import 'package:gui_flutter/widgets/library/track_list_pane.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:material_ui/material_ui.dart';

import 'support/mixar_material_app.dart';

/// The folder actions ⋯ sits in the track-list topbar while browsing a drive,
/// in the same place the active collection's menu sits in the collections tab.
/// One folder backs at most one collection, so a collected folder has no menu.
void main() {
  const listing = FsDirectoryListing(
    path: '/tmp/samples/Music',
    parent: '/tmp/samples',
    directories: [],
    audioFiles: [],
  );
  const volume = FsVolumeInfo(name: 'Home', path: '/', isRemovable: false);
  const collected = LibraryCollectionSummary(
    id: 'c1',
    name: 'Music',
    kind: 'folder',
    path: '/tmp/samples/Music',
    trackCount: 3,
  );

  Future<void> pumpDrive(
    WidgetTester tester, {
    required List<LibraryCollectionSummary> collections,
  }) async {
    final theme = MixarThemeData.light();
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          appSettingsProvider.overrideWith((ref) async => defaultAppSettings()),
          collectionsProvider.overrideWith((ref) async => collections),
          driveListingProvider.overrideWith((ref) async => listing),
          driveVolumesProvider.overrideWith((ref) async => const [volume]),
        ],
        child: MaterialApp(
          theme: materialUiThemeFromMixar(theme),
          builder: mixarMaterialAppBuilder(theme),
          home: const Scaffold(body: TrackListPane()),
        ),
      ),
    );
    final container = ProviderScope.containerOf(
      tester.element(find.byType(TrackListPane)),
    );
    container.read(driveCurrentPathProvider.notifier).set(listing.path);
    container
        .read(librarySourceTabProvider.notifier)
        .set(LibrarySourceTab.drive);
    await tester.pumpAndSettle();
  }

  Finder folderActions() => find.bySemanticsLabel('Folder actions');

  testWidgets('the topbar folder menu offers Create collection', (
    tester,
  ) async {
    await pumpDrive(tester, collections: const []);

    expect(folderActions(), findsOneWidget);
    await tester.tap(folderActions());
    await tester.pumpAndSettle();

    expect(find.text('Create collection'), findsOneWidget);
  });

  testWidgets('a folder that backs a collection has no folder menu', (
    tester,
  ) async {
    await pumpDrive(tester, collections: const [collected]);

    expect(folderActions(), findsNothing);
  });
}
