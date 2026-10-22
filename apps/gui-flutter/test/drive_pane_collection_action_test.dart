import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/theme/material_theme.dart';
import 'package:gui_flutter/providers/library/providers.dart';
import 'package:gui_flutter/src/rust/api/fs_browser.dart';
import 'package:gui_flutter/src/rust/api/library.dart';
import 'package:gui_flutter/widgets/library/drive_pane.dart';
import 'package:gui_flutter/widgets/library/library_nav_row.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:material_ui/material_ui.dart';

import 'support/fake_library_transport.dart';
import 'support/mixar_material_app.dart';

/// One folder backs at most one collection, so the drive pane only offers
/// "Create collection" for folders that do not back one yet.
void main() {
  const folders = [
    FsEntry(name: 'a', path: '/tmp/samples/a'),
    FsEntry(name: 'b', path: '/tmp/samples/b'),
  ];
  const listing = FsDirectoryListing(
    path: '/tmp/samples',
    parent: '/tmp',
    directories: folders,
    audioFiles: [],
  );

  Future<void> pumpDrive(
    WidgetTester tester, {
    required List<LibraryCollectionSummary> collections,
  }) async {
    final transport = FakeLibraryTransport();
    addTearDown(transport.dispose);
    final theme = MixarThemeData.light();
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          collectionsProvider.overrideWith((ref) async => collections),
          driveListingProvider.overrideWith((ref) async => listing),
          driveVolumesProvider.overrideWith(
            (ref) async => const [
              FsVolumeInfo(name: 'root', path: '/', isRemovable: false),
            ],
          ),
          libraryTransportProvider.overrideWith((ref) async => transport),
        ],
        child: MaterialApp(
          theme: materialUiThemeFromMixar(theme),
          builder: mixarMaterialAppBuilder(theme),
          home: const Scaffold(
            body: SizedBox(width: 900, height: 600, child: DrivePane()),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();
    final container = ProviderScope.containerOf(
      tester.element(find.byType(DrivePane)),
    );
    container.read(driveCurrentPathProvider.notifier).set(listing.path);
    await tester.pumpAndSettle();
  }

  Finder rowFor(String name) {
    return find.ancestor(
      of: find.text(name),
      matching: find.byType(LibraryNavRow),
    );
  }

  testWidgets('a folder that backs a collection has no create action', (
    tester,
  ) async {
    await pumpDrive(
      tester,
      collections: const [
        LibraryCollectionSummary(
          id: 'c1',
          name: 'a',
          kind: 'folder',
          path: '/tmp/samples/a',
          trackCount: 1,
        ),
      ],
    );

    expect(
      find.descendant(
        of: rowFor('a'),
        matching: find.bySemanticsLabel('Create collection'),
      ),
      findsNothing,
    );
    expect(
      find.descendant(
        of: rowFor('b'),
        matching: find.bySemanticsLabel('Create collection'),
      ),
      findsOneWidget,
    );
  });

  testWidgets('without a collection every folder keeps the create action', (
    tester,
  ) async {
    await pumpDrive(tester, collections: const []);

    expect(
      find.descendant(
        of: rowFor('a'),
        matching: find.bySemanticsLabel('Create collection'),
      ),
      findsOneWidget,
    );
    expect(
      find.descendant(
        of: rowFor('b'),
        matching: find.bySemanticsLabel('Create collection'),
      ),
      findsOneWidget,
    );
  });
}
