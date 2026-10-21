import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_riverpod/misc.dart' show Override;
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/theme/material_theme.dart';
import 'package:gui_flutter/core/utils/library_list_layout.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/providers/library/history_providers.dart';
import 'package:gui_flutter/providers/library/providers.dart';
import 'package:gui_flutter/providers/settings/settings_providers.dart';
import 'package:gui_flutter/screens/mixer_page.dart';
import 'package:gui_flutter/widgets/library/library_nav_row.dart';
import 'package:gui_flutter/src/rust/api/library.dart';
import 'package:gui_flutter/widgets/mixer/library_panel.dart';
import 'package:gui_flutter/widgets/shell/mixar_input.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';
import 'package:material_ui/material_ui.dart';
import 'package:panes/panes.dart';

import 'helpers/collection_tracks_override.dart';

import 'package:gui_flutter/widgets/shell/m_tab_bar.dart';

import 'support/border_helpers.dart';
import 'support/mixar_material_app.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  const collection = LibraryCollectionSummary(
    id: 'c1',
    name: 'samples',
    kind: 'folder',
    path: '/tmp/samples',
    trackCount: 0,
  );

  List<Override> overrides({List<LibraryCollectionSummary>? collections}) => [
    collectionsProvider.overrideWith(
      (ref) async => collections ?? [collection],
    ),
    overrideCollectionTracksValue(const []),
    historySessionsProvider.overrideWith((ref) async => const []),
    historyCanResumeProvider.overrideWith((ref) async => false),
    libraryEventsBootstrapProvider.overrideWith((ref) {}),
    historySettingsBootstrapProvider.overrideWith((ref) {}),
    librarySettingsBootstrapProvider.overrideWith((ref) {}),
    appSettingsProvider.overrideWith((ref) async => defaultAppSettings()),
  ];

  Future<void> pumpSized(
    WidgetTester tester, {
    required Widget child,
    List<LibraryCollectionSummary>? collections,
  }) async {
    final theme = MixarThemeData.dark();
    tester.view.physicalSize = const Size(1200, 800);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);

    await tester.pumpWidget(
      ProviderScope(
        overrides: overrides(collections: collections),
        child: MaterialApp(
          theme: materialUiThemeFromMixar(theme),
          builder: mixarMaterialAppBuilder(theme),
          home: Scaffold(body: child),
        ),
      ),
    );
    await tester.pump();
  }

  testWidgets('LibraryPanel builds MultiPane split', (tester) async {
    await pumpSized(tester, child: const LibraryPanel());
    await tester.pumpAndSettle();
    expect(find.byType(MultiPane), findsOneWidget);
    expect(find.byIcon(LucideIcons.library), findsOneWidget);
  });

  testWidgets('MixerPage builds MultiPane split', (tester) async {
    await pumpSized(tester, child: const MixerPage());
    await tester.pumpAndSettle();
    expect(find.byType(MultiPane), findsWidgets);
    expect(find.text('Load tracks to see waveforms.'), findsOneWidget);
  });

  testWidgets('sidebar tabs and the track toolbar share a header height', (
    tester,
  ) async {
    await pumpSized(tester, child: const LibraryPanel());
    await tester.pumpAndSettle();

    // Both header rows are found by their stable keys, not by a fill colour
    // that `muted` and `secondary` share.
    final tabBar = find.byKey(kMTabBarKey);
    final toolbar = find.byKey(kLibraryToolbarKey);
    expect(tabBar, findsOneWidget);
    expect(toolbar, findsOneWidget);

    // Both rows are the shared header height, not merely equal to each other.
    expect(tester.getSize(tabBar).height, kLibraryHeaderHeight);
    expect(tester.getSize(toolbar).height, kLibraryHeaderHeight);

    // The filter field's band fills the header up to the bottom border, so
    // there is no gap above the field and it does not paint over the border.
    final band = find.byWidgetPredicate(
      (w) =>
          w is ColoredBox &&
          w.color == libraryToolbarFieldColor(MixarThemeData.dark()),
    );
    expect(band, findsOneWidget);
    expect(
      tester.getSize(band).height,
      kLibraryHeaderHeight - MixarThemeData.dark().style.borderWidth,
      reason: 'the field band must fill the header up to the bottom border',
    );

    // Both header rows draw the same bottom border, so the rule reads as one
    // continuous line across the split.
    expect(
      borderOf(tester, tabBar).bottom.color,
      borderOf(tester, toolbar).bottom.color,
      reason: 'the tabs and topbar bottom borders must match',
    );
    expect(
      borderOf(tester, toolbar).bottom.width,
      MixarThemeData.dark().style.borderWidth,
      reason: 'the toolbar border uses the theme hairline width',
    );
  });

  testWidgets('a focus step moves the accent onto the sidebar cursor', (
    tester,
  ) async {
    await pumpSized(tester, child: const LibraryPanel());
    await tester.pumpAndSettle();
    final theme = MixarThemeData.dark();
    // The selected collection row: it is the sidebar cursor while the sidebar
    // owns browser focus.
    final row = find.ancestor(
      of: find.text('samples'),
      matching: find.byType(LibraryNavRow),
    );
    expect(row, findsOneWidget);
    final rowBox = find
        .descendant(of: row, matching: find.byType(DecoratedBox))
        .first;

    // Track list owns focus on open: the row carries the selection fill and the
    // left accent only.
    expect(fillOf(tester, rowBox), theme.colors.selection);
    expect(borderOf(tester, rowBox).left.color, theme.colors.primary);
    expect(borderOf(tester, rowBox).left.width, 2);

    final container = ProviderScope.containerOf(
      tester.element(find.byType(LibraryPanel)),
    );
    container
        .read(libraryFocusZoneProvider.notifier)
        .set(LibraryFocusZone.sidebar);
    await tester.pumpAndSettle();

    // Focused row: the left accent grows into a full outline, so a selected row
    // and a focused row no longer read the same.
    expect(borderOf(tester, rowBox).top.color, theme.colors.primary);
    expect(borderOf(tester, rowBox).top.width, 2);
    expect(fillOf(tester, rowBox), theme.colors.selection);

    container
        .read(libraryFocusZoneProvider.notifier)
        .set(LibraryFocusZone.trackList);
    await tester.pumpAndSettle();
    expect(borderOf(tester, rowBox).top.color, isNot(theme.colors.primary));
    expect(borderOf(tester, rowBox).left.color, theme.colors.primary);
    expect(fillOf(tester, rowBox), theme.colors.selection);
  });

  testWidgets('the sidebar cursor scrolls its row into view', (tester) async {
    // Far more collections than the sidebar can show, so the cursor can leave
    // the fold.
    final many = [
      for (var i = 0; i < 40; i++)
        LibraryCollectionSummary(
          id: 'c$i',
          name: 'col $i',
          kind: 'folder',
          trackCount: 0,
        ),
    ];
    await pumpSized(tester, child: const LibraryPanel(), collections: many);
    await tester.pumpAndSettle();
    final container = ProviderScope.containerOf(
      tester.element(find.byType(LibraryPanel)),
    );
    container
        .read(libraryFocusZoneProvider.notifier)
        .set(LibraryFocusZone.sidebar);

    // Walk the cursor one row at a time, the way the rotary does, until the row
    // it lands on is past the fold.
    for (var i = 1; i <= 20; i++) {
      container.read(selectedCollectionIdProvider.notifier).set('c$i');
      await tester.pumpAndSettle();
    }

    // The cursor's row is inside the viewport, and the list scrolled to get it
    // there rather than leaving it below the fold.
    final rowTop = tester.getTopLeft(find.text('col 20')).dy;
    expect(rowTop, greaterThanOrEqualTo(0));
    expect(rowTop, lessThan(tester.view.physicalSize.height));
    final scroller = tester
        .state<ScrollableState>(
          find
              .ancestor(
                of: find.text('col 20'),
                matching: find.byType(Scrollable),
              )
              .first,
        )
        .position;
    expect(scroller.pixels, greaterThan(0));
  });

  testWidgets('the filter band darkens while the field is focused', (
    tester,
  ) async {
    await pumpSized(tester, child: const LibraryPanel());
    await tester.pumpAndSettle();

    final theme = MixarThemeData.dark();
    Finder band({required bool focused}) => find.byWidgetPredicate(
      (w) =>
          w is ColoredBox &&
          w.color == libraryToolbarFieldColor(theme, focused: focused),
    );

    expect(band(focused: false), findsOneWidget);
    expect(band(focused: true), findsNothing);

    await tester.tap(find.byType(MixarInput));
    await tester.pumpAndSettle();

    expect(band(focused: true), findsOneWidget);
    expect(band(focused: false), findsNothing);
  });
}
