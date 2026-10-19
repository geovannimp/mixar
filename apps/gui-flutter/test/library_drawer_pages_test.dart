import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_riverpod/misc.dart' show Override;
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/models/shell_tab.dart';
import 'package:gui_flutter/core/theme/material_theme.dart';
import 'package:gui_flutter/core/utils/library_list_layout.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/providers/library/drawer_providers.dart';
import 'package:gui_flutter/providers/library/history_providers.dart';
import 'package:gui_flutter/providers/library/providers.dart';
import 'package:gui_flutter/providers/settings/settings_providers.dart';
import 'package:gui_flutter/screens/mixer_page.dart';
import 'package:gui_flutter/src/rust/api/library.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';
import 'package:gui_flutter/widgets/mixer/library_panel.dart';
import 'package:gui_flutter/widgets/shell/app_header.dart';
import 'package:gui_flutter/widgets/shell/m_tab_bar.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';
import 'package:material_ui/material_ui.dart';
import 'package:panes/panes.dart';

import 'helpers/collection_tracks_override.dart';

import 'support/mixar_material_app.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  const collection = LibraryCollectionSummary(
    id: 'c1',
    name: 'samples',
    kind: 'folder',
    path: '/tmp/samples',
    trackCount: 1,
  );
  const track = LibraryTrackSummary(
    id: 't1',
    displayName: 'Demo Track',
    artist: 'Artist',
    title: 'Demo Track',
    bpm: 128,
    key: '8A',
    durationMs: 180000,
    path: '/tmp/samples/demo.wav',
  );

  List<Override> overrides({AppSettings? settings}) => [
    collectionsProvider.overrideWith((ref) async => [collection]),
    overrideCollectionTracksValue(const [track]),
    historySessionsProvider.overrideWith((ref) async => const []),
    historyCanResumeProvider.overrideWith((ref) async => false),
    libraryEventsBootstrapProvider.overrideWith((ref) {}),
    historySettingsBootstrapProvider.overrideWith((ref) {}),
    librarySettingsBootstrapProvider.overrideWith((ref) {}),
    appSettingsProvider.overrideWith(
      (ref) async => settings ?? defaultAppSettings(),
    ),
  ];

  Future<void> pumpSized(
    WidgetTester tester, {
    required Widget child,
    Size size = const Size(800, 600),
    AppSettings? settings,
  }) async {
    final theme = MixarThemeData.dark();
    tester.view.physicalSize = size;
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);

    await tester.pumpWidget(
      ProviderScope(
        overrides: overrides(settings: settings),
        child: mixarTestMaterialApp(
          theme: theme,
          home: Scaffold(body: child),
        ),
      ),
    );
    await tester.pumpAndSettle();
  }

  testWidgets('narrow library pages the sidebar then the track list', (
    tester,
  ) async {
    await pumpSized(
      tester,
      child: const SizedBox(width: 480, height: 600, child: LibraryPanel()),
    );

    // Below the split threshold the sidebar page shows alone.
    expect(find.byType(MultiPane), findsNothing);
    expect(find.byKey(kMTabBarKey), findsOneWidget);
    expect(find.byKey(kLibraryToolbarKey), findsNothing);
    expect(find.text('samples'), findsOneWidget);

    await tester.tap(find.text('samples'));
    await tester.pumpAndSettle();

    // The content page replaces it, with the back button left of the filter.
    expect(find.byKey(kMTabBarKey), findsNothing);
    expect(find.byKey(kLibraryToolbarKey), findsOneWidget);
    expect(find.text('Demo Track'), findsOneWidget);
    expect(find.byIcon(LucideIcons.arrowLeft), findsOneWidget);

    await tester.tap(find.byIcon(LucideIcons.arrowLeft));
    await tester.pumpAndSettle();

    expect(find.byKey(kMTabBarKey), findsOneWidget);
    expect(find.byKey(kLibraryToolbarKey), findsNothing);
    expect(find.byIcon(LucideIcons.arrowLeft), findsNothing);
  });

  testWidgets('wide library keeps the split and no back button', (
    tester,
  ) async {
    await pumpSized(
      tester,
      // The viewport must be as wide as the surface, as it is in the app:
      // a `SizedBox` wider than the viewport is constrained by it.
      size: const Size(1200, 600),
      child: const SizedBox(width: 1200, height: 600, child: LibraryPanel()),
    );

    expect(find.byType(MultiPane), findsOneWidget);
    expect(find.byKey(kMTabBarKey), findsOneWidget);
    expect(find.byKey(kLibraryToolbarKey), findsOneWidget);
    expect(find.text('Demo Track'), findsOneWidget);
    expect(find.byIcon(LucideIcons.arrowLeft), findsNothing);
  });

  testWidgets('drawer mode hides the docked panel and slides the drawer', (
    tester,
  ) async {
    await pumpSized(
      tester,
      size: const Size(1200, 800),
      settings: copyAppSettings(
        defaultAppSettings(),
        libraryDrawerEnabled: true,
        libraryDrawerSide: LibraryDrawerSideSetting.left,
      ),
      child: const MixerPage(),
    );

    // Drawer mode: the docked panel is gone and the drawer starts closed.
    expect(find.byType(LibraryPanel), findsNothing);

    final container = ProviderScope.containerOf(
      tester.element(find.byType(MixerPage)),
    );
    container.read(libraryDrawerOpenProvider.notifier).set(true);
    await tester.pumpAndSettle();

    // A 480px drawer is narrower than the split threshold, so the list page
    // shows. It is flush to its edge.
    expect(find.byType(LibraryPanel), findsOneWidget);
    // Scoped to the drawer: the mixer's own tabs (waveform lanes) share the key.
    expect(
      find.descendant(
        of: find.byType(LibraryPanel),
        matching: find.byKey(kMTabBarKey),
      ),
      findsOneWidget,
    );
    expect(find.byKey(kLibraryToolbarKey), findsNothing);
    expect(tester.getTopLeft(find.byType(LibraryPanel)).dx, 0);

    // A track drag slides it off its edge but keeps it mounted, so the drag
    // source survives.
    container.read(libraryTrackDragActiveProvider.notifier).begin();
    await tester.pumpAndSettle();
    expect(find.byType(LibraryPanel), findsOneWidget);
    expect(tester.getTopLeft(find.byType(LibraryPanel)).dx, lessThan(0));

    container.read(libraryTrackDragActiveProvider.notifier).end();
    await tester.pumpAndSettle();
    expect(tester.getTopLeft(find.byType(LibraryPanel)).dx, 0);
  });

  testWidgets('the header button toggles the drawer in drawer mode', (
    tester,
  ) async {
    await pumpSized(
      tester,
      settings: copyAppSettings(
        defaultAppSettings(),
        libraryDrawerEnabled: true,
        libraryDrawerSide: LibraryDrawerSideSetting.left,
      ),
      child: AppHeader(
        appTitle: 'Mixar',
        tab: ShellTab.mixer,
        onTabChanged: (_) {},
      ),
    );

    // The header renders on its own, so scope to it: the mixer's own chrome
    // uses other Lucide icons.
    final button = find.descendant(
      of: find.byType(AppHeader),
      matching: find.byIcon(LucideIcons.libraryBig),
    );
    expect(button, findsOneWidget);
    final container = ProviderScope.containerOf(
      tester.element(find.byType(AppHeader)),
    );
    expect(container.read(libraryDrawerOpenProvider), isFalse);

    await tester.tap(button);
    await tester.pumpAndSettle();
    expect(container.read(libraryDrawerOpenProvider), isTrue);

    await tester.tap(button);
    await tester.pumpAndSettle();
    expect(container.read(libraryDrawerOpenProvider), isFalse);
  });

  testWidgets('the header button is absent with the drawer off', (
    tester,
  ) async {
    await pumpSized(
      tester,
      child: AppHeader(
        appTitle: 'Mixar',
        tab: ShellTab.mixer,
        onTabChanged: (_) {},
      ),
    );
    expect(
      find.descendant(
        of: find.byType(AppHeader),
        matching: find.byIcon(LucideIcons.libraryBig),
      ),
      findsNothing,
    );
  });

  testWidgets('the header button is absent off the mixer', (tester) async {
    await pumpSized(
      tester,
      settings: copyAppSettings(
        defaultAppSettings(),
        libraryDrawerEnabled: true,
      ),
      child: AppHeader(
        appTitle: 'Mixar',
        tab: ShellTab.settings,
        onTabChanged: (_) {},
      ),
    );
    expect(
      find.descendant(
        of: find.byType(AppHeader),
        matching: find.byIcon(LucideIcons.libraryBig),
      ),
      findsNothing,
    );
  });

  testWidgets('the drawer resizes from its inner edge, clamped to the minimum', (
    tester,
  ) async {
    await pumpSized(
      tester,
      size: const Size(1200, 800),
      settings: copyAppSettings(
        defaultAppSettings(),
        libraryDrawerEnabled: true,
        libraryDrawerSide: LibraryDrawerSideSetting.left,
      ),
      child: const MixerPage(),
    );

    final container = ProviderScope.containerOf(
      tester.element(find.byType(MixerPage)),
    );
    container.read(libraryDrawerOpenProvider.notifier).set(true);
    await tester.pumpAndSettle();
    expect(container.read(libraryDrawerSizeProvider).width, 480);
    expect(tester.getSize(find.byType(LibraryPanel)).width, 480);

    // The grip is an 8px strip on the drawer's inner edge, so grab just inside
    // it. A left drawer widens when dragged right.
    Future<void> dragGrip(double dx) async {
      final gripX = tester.getTopRight(find.byType(LibraryPanel)).dx - 4;
      await tester.dragFrom(Offset(gripX, 300), Offset(dx, 0));
      await tester.pumpAndSettle();
    }

    await dragGrip(80);
    final widened = container.read(libraryDrawerSizeProvider).width;
    expect(widened, greaterThan(480));
    expect(tester.getSize(find.byType(LibraryPanel)).width, widened);

    // Dragged past its minimum it stops there instead of collapsing.
    await dragGrip(-2000);
    expect(
      container.read(libraryDrawerSizeProvider).width,
      kLibraryDrawerMinExtent,
    );
    expect(
      tester.getSize(find.byType(LibraryPanel)).width,
      kLibraryDrawerMinExtent,
    );
  });

  testWidgets('the default bottom drawer docks full width and resizes', (
    tester,
  ) async {
    await pumpSized(
      tester,
      size: const Size(1200, 800),
      settings: copyAppSettings(
        defaultAppSettings(),
        libraryDrawerEnabled: true,
      ),
      child: const MixerPage(),
    );

    final container = ProviderScope.containerOf(
      tester.element(find.byType(MixerPage)),
    );
    container.read(libraryDrawerOpenProvider.notifier).set(true);
    await tester.pumpAndSettle();

    // Bottom is the default side: 480px tall and full width, so the surface is
    // wider than the split threshold and shows the split.
    final panel = tester.getRect(find.byType(LibraryPanel));
    expect(panel.height, 480);
    expect(panel.width, 1200);
    expect(panel.left, 0);
    expect(panel.bottom, 800);
    // Scoped: the mixer page's own vertical split shares the type.
    expect(
      find.descendant(
        of: find.byType(LibraryPanel),
        matching: find.byType(MultiPane),
      ),
      findsOneWidget,
    );
    expect(find.byKey(kLibraryToolbarKey), findsOneWidget);

    // The grip is the top edge; dragging it up grows the drawer.
    final gripY = panel.top + 4;
    await tester.dragFrom(Offset(600, gripY), const Offset(0, -80));
    await tester.pumpAndSettle();
    final grown = container.read(libraryDrawerSizeProvider).height;
    expect(grown, greaterThan(480));
    expect(tester.getSize(find.byType(LibraryPanel)).height, grown);
  });
}
