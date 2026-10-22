import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/providers/library/providers.dart';
import 'package:gui_flutter/providers/shell/app_notice.dart';
import 'package:gui_flutter/src/rust/api/library.dart';
import 'package:gui_flutter/widgets/library/collection_actions_menu.dart';
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

FakeLibraryTransport _newTransport({Object? deleteCollectionError}) {
  final transport = FakeLibraryTransport(
    collections: const [_playlist],
    deleteCollectionError: deleteCollectionError,
  );
  addTearDown(transport.dispose);
  return transport;
}

/// Mount the collection's three-dot menu with [_playlist] active.
Future<ProviderContainer> _pumpMenu(
  WidgetTester tester, {
  required FakeLibraryTransport transport,
}) async {
  tester.view.physicalSize = const Size(1000, 1200);
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
  return ProviderScope.containerOf(
    tester.element(find.byType(CollectionActionsMenu)),
  );
}

Future<void> _openDeleteConfirm(WidgetTester tester) async {
  await tester.tap(find.byIcon(LucideIcons.ellipsisVertical));
  await tester.pumpAndSettle();
  await tester.tap(find.text('Delete'));
  await tester.pumpAndSettle();
}

void main() {
  testWidgets('delete asks for confirmation then deletes the collection', (
    tester,
  ) async {
    final transport = _newTransport();
    final container = await _pumpMenu(tester, transport: transport);
    container.read(selectedCollectionIdProvider.notifier).set(_playlist.id);

    await _openDeleteConfirm(tester);
    expect(find.text('Delete this collection?'), findsOneWidget);
    final bodyText = tester
        .widget<Text>(find.textContaining('Original audio files on disk'))
        .data!;
    expect('• '.allMatches(bodyText).length, 2);
    expect(
      bodyText,
      contains('Tracks only used by this collection are removed too.'),
    );
    expect(transport.deleteCollectionCalls, isEmpty);

    await tester.tap(find.text('Delete'));
    await tester.pumpAndSettle();

    expect(find.text('Delete this collection?'), findsNothing);
    expect(transport.deleteCollectionCalls, [_playlist.id]);
    expect(container.read(selectedCollectionIdProvider), isNull);
  }, semanticsEnabled: false);

  testWidgets('a failed cleanup still refreshes the collection state', (
    tester,
  ) async {
    final transport = _newTransport(
      deleteCollectionError: Exception('cleanup failed'),
    );
    final container = await _pumpMenu(tester, transport: transport);
    container.read(selectedCollectionIdProvider.notifier).set(_playlist.id);
    await container.read(collectionsProvider.future);
    final loadsBeforeDelete = transport.listCollectionsCalls;

    await _openDeleteConfirm(tester);
    await tester.tap(find.text('Delete'));
    await tester.pumpAndSettle();

    expect(transport.deleteCollectionCalls, [_playlist.id]);
    expect(transport.collections, isEmpty);
    expect(container.read(selectedCollectionIdProvider), isNull);
    expect(container.read(appNoticeProvider)?.kind, AppNoticeKind.error);
    // Invalidation drops the cached list; the next read must re-query.
    await container.read(collectionsProvider.future);
    expect(transport.listCollectionsCalls, greaterThan(loadsBeforeDelete));
  }, semanticsEnabled: false);

  testWidgets('cancelling the confirmation keeps the collection', (
    tester,
  ) async {
    final transport = _newTransport();
    await _pumpMenu(tester, transport: transport);

    await _openDeleteConfirm(tester);
    await tester.tap(find.text('Cancel'));
    await tester.pumpAndSettle();

    expect(find.text('Delete this collection?'), findsNothing);
    expect(transport.deleteCollectionCalls, isEmpty);
  }, semanticsEnabled: false);
}
