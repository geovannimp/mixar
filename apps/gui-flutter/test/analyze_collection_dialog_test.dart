import 'dart:async';

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/library/analyze_collection_dialog.dart';
import 'package:gui_flutter/library/collection_actions_menu.dart';
import 'package:gui_flutter/shell/material_theme.dart';
import 'package:gui_flutter/shell/mixar_switch.dart';
import 'package:gui_flutter/shell/mixar_theme.dart';
import 'package:gui_flutter/src/rust/api/library.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';
import 'package:material_ui/material_ui.dart';

import 'support/mixar_material_app.dart';

const _folder = LibraryCollectionSummary(
  id: 'folder:1',
  name: 'STEMS FINAL',
  kind: 'folder',
  path: '/music/stems',
  trackCount: 65,
);

const _playlist = LibraryCollectionSummary(
  id: 'playlist:1',
  name: 'Set',
  kind: 'playlist',
  trackCount: 3,
);

Future<void> pumpMenu(
  WidgetTester tester,
  LibraryCollectionSummary collection,
) async {
  final theme = MixarThemeData.dark();
  await tester.pumpWidget(
    ProviderScope(
      child: MaterialApp(
        theme: materialUiThemeFromMixar(theme),
        builder: mixarMaterialAppBuilder(theme),
        home: Scaffold(
          body: Center(child: CollectionActionsMenu(collection: collection)),
        ),
      ),
    ),
  );
  await tester.pumpAndSettle();
}

void main() {
  testWidgets('folder menu offers browse and analyze', (tester) async {
    await pumpMenu(tester, _folder);
    await tester.tap(find.byIcon(LucideIcons.ellipsisVertical));
    await tester.pumpAndSettle();
    expect(find.text('Browse in Drive'), findsOneWidget);
    expect(find.text('Analyze tracks…'), findsOneWidget);
  });

  testWidgets('playlist menu offers analyze only', (tester) async {
    await pumpMenu(tester, _playlist);
    await tester.tap(find.byIcon(LucideIcons.ellipsisVertical));
    await tester.pumpAndSettle();
    expect(find.text('Browse in Drive'), findsNothing);
    expect(find.text('Analyze tracks…'), findsOneWidget);
  });

  testWidgets('analyze opens the options dialog with skip-first copy', (
    tester,
  ) async {
    await pumpMenu(tester, _folder);
    await tester.tap(find.byIcon(LucideIcons.ellipsisVertical));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Analyze tracks…'));
    await tester.pumpAndSettle();
    expect(find.text('Analyze STEMS FINAL'), findsOneWidget);
    expect(find.text('Force re-analysis'), findsOneWidget);
    expect(find.text('Also generate stems'), findsOneWidget);
    expect(find.text('Start analysis'), findsOneWidget);
  });

  testWidgets('dialog returns force + stems choices', (tester) async {
    AnalyzeCollectionOptions? result;
    final theme = MixarThemeData.dark();
    await tester.pumpWidget(
      MaterialApp(
        theme: materialUiThemeFromMixar(theme),
        builder: mixarMaterialAppBuilder(theme),
        home: const Scaffold(body: SizedBox.shrink()),
      ),
    );
    final context = tester.element(find.byType(Scaffold));
    unawaited(
      showAnalyzeCollectionDialog(
        context,
        collectionName: 'STEMS FINAL',
        trackCount: 65,
      ).then((value) => result = value),
    );
    await tester.pumpAndSettle();
    final switches = find.byType(MixarSwitch);
    expect(switches, findsNWidgets(2));
    await tester.tap(switches.at(0));
    await tester.pump();
    await tester.tap(switches.at(1));
    await tester.pump();
    await tester.tap(find.text('Start analysis'));
    await tester.pumpAndSettle();
    expect(result, isNotNull);
    expect(result!.force, isTrue);
    expect(result!.generateStems, isTrue);
  });
}
