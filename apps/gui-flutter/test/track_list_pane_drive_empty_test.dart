import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/theme/material_theme.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/providers/library/providers.dart';
import 'package:gui_flutter/providers/settings/settings_providers.dart';
import 'package:gui_flutter/src/rust/api/fs_browser.dart';
import 'package:gui_flutter/widgets/library/track_list_pane.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:material_ui/material_ui.dart';

import 'support/mixar_material_app.dart';

/// A drive folder with nothing to play names the folder the sidebar names it
/// by, not "this folder": the user may be several folders deep.
void main() {
  const listing = FsDirectoryListing(
    path: '/tmp/samples/Music',
    parent: '/tmp/samples',
    directories: [],
    audioFiles: [],
  );
  const volume = FsVolumeInfo(name: 'Home', path: '/', isRemovable: false);

  Future<void> pumpDrive(
    WidgetTester tester, {
    required String path,
    required FsDirectoryListing emptyListing,
  }) async {
    final theme = MixarThemeData.light();
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          appSettingsProvider.overrideWith((ref) async => defaultAppSettings()),
          driveListingProvider.overrideWith((ref) async => emptyListing),
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
    container.read(driveCurrentPathProvider.notifier).set(path);
    container
        .read(librarySourceTabProvider.notifier)
        .set(LibrarySourceTab.drive);
    await tester.pumpAndSettle();
  }

  testWidgets('an empty folder is named by its last path segment', (
    tester,
  ) async {
    await pumpDrive(tester, path: listing.path, emptyListing: listing);

    expect(find.text('No audio files in “Music”'), findsOneWidget);
  });

  testWidgets('an empty volume root is named by the volume', (tester) async {
    const root = FsDirectoryListing(path: '/', directories: [], audioFiles: []);
    await pumpDrive(tester, path: root.path, emptyListing: root);

    expect(find.text('No audio files in “Home”'), findsOneWidget);
  });
}
