import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/providers/library/providers.dart';
import 'package:gui_flutter/providers/shell/app_notice.dart';
import 'package:gui_flutter/src/rust/api/library.dart';
import 'package:gui_flutter/widgets/library/collection_actions_menu.dart';
import 'package:gui_flutter/widgets/library/smart_rules_editor.dart';
import 'package:gui_flutter/widgets/shell/app_button.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';
import 'package:material_ui/material_ui.dart';
import 'package:shadcn_ui/shadcn_ui.dart';

import 'support/fake_library_transport.dart';
import 'support/mixar_material_app.dart';

const _playlist = LibraryCollectionSummary(
  id: 'pl-warmup',
  name: 'Warmup',
  kind: 'playlist',
  trackCount: 3,
);

const _smart = LibraryCollectionSummary(
  id: 'smart-1',
  name: '±6% + key',
  kind: 'smart_playlist',
);

FakeLibraryTransport _newTransport({Object? deleteCollectionError}) {
  final transport = FakeLibraryTransport(
    collections: const [_playlist],
    deleteCollectionError: deleteCollectionError,
  );
  addTearDown(transport.dispose);
  return transport;
}

/// Mount the collection's three-dot menu with [collection] active.
Future<ProviderContainer> _pumpMenu(
  WidgetTester tester, {
  required FakeLibraryTransport transport,
  LibraryCollectionSummary collection = _playlist,
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
        home: Scaffold(
          body: Center(child: CollectionActionsMenu(collection: collection)),
        ),
      ),
    ),
  );
  await tester.pumpAndSettle();
  return ProviderScope.containerOf(
    tester.element(find.byType(CollectionActionsMenu)),
  );
}

Future<void> _openMenu(WidgetTester tester) async {
  await tester.tap(find.byIcon(LucideIcons.ellipsisVertical));
  await tester.pumpAndSettle();
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

  testWidgets('a manual playlist has no rules to edit', (tester) async {
    final transport = _newTransport();
    await _pumpMenu(tester, transport: transport);

    await _openMenu(tester);

    expect(find.text('Edit rules…'), findsNothing);
    expect(find.text('Analyze tracks…'), findsOneWidget);
  }, semanticsEnabled: false);

  testWidgets('a smart playlist offers Edit rules… and saves the document', (
    tester,
  ) async {
    const stored =
        '{"version":1,"root":{"match":"all","rules":['
        '{"kind":"condition","field":"rating","op":"is_empty"}]}}';
    final transport = FakeLibraryTransport(collections: const [_smart]);
    addTearDown(transport.dispose);
    transport.smartPlaylistRulesHandler = (_) async => stored;
    await _pumpMenu(tester, transport: transport, collection: _smart);

    await _openMenu(tester);
    expect(find.text('Edit rules…'), findsOneWidget);
    await tester.tap(find.text('Edit rules…'));
    await tester.pumpAndSettle();

    expect(transport.smartPlaylistRulesCalls, [_smart.id]);
    expect(find.text('Smart playlist'), findsOneWidget);

    await tester.tap(find.text('Save'));
    await tester.pumpAndSettle();

    final call = transport.updateSmartPlaylistCalls.single;
    expect(call.collectionId, _smart.id);
    expect(call.name, _smart.name);
    expect(call.rulesJson, stored);
  }, semanticsEnabled: false);

  testWidgets('clearing a smart playlist’s rules blocks Save', (tester) async {
    const stored =
        '{"version":1,"root":{"match":"all","rules":['
        '{"kind":"condition","field":"rating","op":"is_empty"}]}}';
    final transport = FakeLibraryTransport(collections: const [_smart]);
    addTearDown(transport.dispose);
    transport.smartPlaylistRulesHandler = (_) async => stored;
    await _pumpMenu(tester, transport: transport, collection: _smart);

    await _openMenu(tester);
    await tester.tap(find.text('Edit rules…'));
    await tester.pumpAndSettle();

    final save = find.widgetWithText(AppButton, 'Save');
    expect(tester.widget<AppButton>(save).onPress, isNotNull);

    // Applying a preset replaces the tree; the prompt then clears it again, and
    // a cleared tree cannot be saved.
    final preset = find
        .descendant(
          of: find.byType(SmartRulesEditor),
          matching: find.byType(ShadSelect<String>),
        )
        .first;
    await tester.tap(preset);
    await tester.pumpAndSettle();
    await tester.tap(find.text('Peak time').last);
    await tester.pumpAndSettle();
    await tester.tap(preset);
    await tester.pumpAndSettle();
    await tester.tap(find.text('Choose a preset').last);
    await tester.pumpAndSettle();

    expect(tester.widget<AppButton>(save).onPress, isNull);
    expect(transport.updateSmartPlaylistCalls, isEmpty);
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
