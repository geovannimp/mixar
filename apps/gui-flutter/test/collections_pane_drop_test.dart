import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/utils/library_list_layout.dart';
import 'package:gui_flutter/core/utils/track_drag.dart';
import 'package:gui_flutter/providers/library/providers.dart';
import 'package:gui_flutter/src/rust/api/library.dart';
import 'package:gui_flutter/widgets/library/collections_pane.dart';
import 'package:gui_flutter/widgets/library/library_nav_row.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';
import 'package:material_ui/material_ui.dart';

import 'support/fake_library_transport.dart';
import 'support/mixar_material_app.dart';

/// Playlist rows are drop targets for a library track; folder rows are not.
void main() {
  const collections = [
    LibraryCollectionSummary(
      id: 'p1',
      name: 'Set A',
      kind: 'playlist',
      trackCount: 0,
    ),
    LibraryCollectionSummary(
      id: 'f1',
      name: 'Folder A',
      kind: 'folder',
      path: '/tmp/samples/a',
      trackCount: 3,
    ),
  ];
  const payload = TrackDragPayload(
    source: TrackDragSource.library,
    trackId: 't1',
    path: '/music/one.flac',
    title: 'One',
  );
  final theme = MixarThemeData.light();

  Future<({FakeLibraryTransport transport, int Function() loads})> pumpPane(
    WidgetTester tester, {
    Object? addError,
  }) async {
    final transport = FakeLibraryTransport(collections: collections)
      ..addCollectionEntryError = addError;
    addTearDown(transport.dispose);
    var collectionsLoads = 0;
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          libraryTransportProvider.overrideWith((ref) async => transport),
          collectionsProvider.overrideWith((ref) async {
            collectionsLoads++;
            return collections;
          }),
        ],
        child: mixarTestMaterialApp(
          theme: theme,
          home: Scaffold(
            body: SizedBox(
              width: 360,
              height: 600,
              child: Stack(
                children: [
                  const Positioned.fill(child: CollectionsPane()),
                  Positioned(
                    left: 0,
                    bottom: 0,
                    child: libraryTrackDragSource(
                      payload: payload,
                      child: const SizedBox(
                        key: Key('drag-source'),
                        width: 80,
                        height: 24,
                      ),
                    ),
                  ),
                ],
              ),
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();
    return (transport: transport, loads: () => collectionsLoads);
  }

  Finder rowFor(String name) =>
      find.ancestor(of: find.text(name), matching: find.byType(LibraryNavRow));

  /// Top border width of the row's own chrome: 2 while a valid payload is over
  /// a drop target, 0 otherwise (the resting row only has a left hairline).
  double topBorderWidth(WidgetTester tester, String name) {
    final box = tester.widget<DecoratedBox>(
      find
          .descendant(of: rowFor(name), matching: find.byType(DecoratedBox))
          .first,
    );
    final border = (box.decoration as BoxDecoration).border;
    return border is Border ? border.top.width : 0;
  }

  /// Press the drag source, carry the payload over [name], measure the row
  /// chrome while the payload is still over it, then release the drop.
  Future<double> dragOver(WidgetTester tester, String name) async {
    final from = tester.getCenter(find.byKey(const Key('drag-source')));
    final gesture = await tester.startGesture(from);
    await tester.pump(const Duration(milliseconds: 50));
    // Past the touch slop, so the drag starts before aiming at the row.
    await gesture.moveBy(const Offset(0, -30));
    await tester.pump();
    await gesture.moveTo(tester.getCenter(rowFor(name)));
    await tester.pump();
    final hoverBorder = topBorderWidth(tester, name);
    await gesture.up();
    await tester.pumpAndSettle();
    return hoverBorder;
  }

  testWidgets('dropping a track on a playlist row adds it to that playlist', (
    tester,
  ) async {
    final pane = await pumpPane(tester);

    expect(topBorderWidth(tester, 'Set A'), 0);
    // The row shows the drop outline while the payload is over it.
    expect(await dragOver(tester, 'Set A'), 2);

    expect(pane.transport.addCollectionEntryCalls, [
      (collectionId: 'p1', trackId: 't1', position: null),
    ]);
    expect(find.text('Added “One” to “Set A”'), findsOneWidget);
    // The sidebar reloads its counts after the add.
    expect(pane.loads(), greaterThan(1));
  });

  testWidgets('playlist rows use the playlist icon, folder rows the folder', (
    tester,
  ) async {
    await pumpPane(tester);

    expect(
      find.descendant(
        of: rowFor('Set A'),
        matching: find.byIcon(LucideIcons.listMusic),
      ),
      findsOneWidget,
    );
    expect(
      find.descendant(
        of: rowFor('Folder A'),
        matching: find.byIcon(LucideIcons.folder),
      ),
      findsOneWidget,
    );
    expect(
      find.descendant(
        of: rowFor('Set A'),
        matching: find.byIcon(LucideIcons.folder),
      ),
      findsNothing,
    );
  });

  testWidgets('a folder row takes no drop and adds nothing', (tester) async {
    final pane = await pumpPane(tester);

    expect(await dragOver(tester, 'Folder A'), 0);

    expect(pane.transport.addCollectionEntryCalls, isEmpty);
    expect(find.textContaining('Added'), findsNothing);
  });

  testWidgets('a failed add reports the error and keeps the rows unchanged', (
    tester,
  ) async {
    final pane = await pumpPane(
      tester,
      addError: Exception('library is read-only'),
    );

    await dragOver(tester, 'Set A');

    expect(find.textContaining('library is read-only'), findsOneWidget);
    expect(find.text('0 tracks'), findsOneWidget);
    // Nothing changed, so the sidebar was not reloaded.
    expect(pane.loads(), 1);
  });
}
