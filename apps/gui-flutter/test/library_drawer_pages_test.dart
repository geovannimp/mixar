import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_riverpod/misc.dart' show Override;
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/models/shell_tab.dart';
import 'package:gui_flutter/core/utils/desktop.dart';
import 'package:gui_flutter/core/utils/library_list_layout.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/core/utils/track_drag.dart';
import 'package:gui_flutter/providers/library/drawer_providers.dart';
import 'package:gui_flutter/providers/library/history_providers.dart';
import 'package:gui_flutter/providers/library/providers.dart';
import 'package:gui_flutter/providers/mixer/engine_providers.dart';
import 'package:gui_flutter/providers/settings/settings_providers.dart';
import 'package:gui_flutter/screens/mixer_page.dart';
import 'package:gui_flutter/src/rust/api/event_mirrors.dart' as bus;
import 'package:gui_flutter/src/rust/api/library.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';
import 'package:gui_flutter/widgets/library/library_pane_back_button.dart';
import 'package:gui_flutter/widgets/library/library_row_density_button.dart';
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
    album: 'Album X',
    genre: 'Genre Y',
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
    List<Override> extra = const [],
  }) async {
    // No native audio or MIDI: the engine/controller providers stay null (their
    // documented widget-test contract) instead of calling the Rust bridge.
    debugOverrideDesktopWindow = false;
    addTearDown(() => debugOverrideDesktopWindow = null);
    final theme = MixarThemeData.dark();
    tester.view.physicalSize = size;
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);

    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          ...overrides(settings: settings),
          ...extra,
        ],
        child: mixarTestMaterialApp(
          theme: theme,
          home: Scaffold(body: child),
        ),
      ),
    );
    await tester.pumpAndSettle();
  }

  /// The container behind the page under test.
  ProviderContainer mixerContainer(WidgetTester tester) =>
      ProviderScope.containerOf(tester.element(find.byType(MixerPage)));

  test(
    'a controller focus step opens the drawer, SHIFT + press closes it',
    () async {
      Future<ProviderContainer> boot(AppSettings settings) async {
        final container = ProviderContainer(
          overrides: [
            appSettingsProvider.overrideWith((ref) async => settings),
          ],
        );
        addTearDown(container.dispose);
        // The drawer's decision reads the resolved settings, not the loading
        // state.
        await container.read(appSettingsProvider.future);
        return container;
      }

      // The DDJ-400's library knob: press is focus_next, SHIFT + press
      // focus_prev.
      const press = bus.Event.focus(bus.Focus(delta: 1));
      const shiftPress = bus.Event.focus(bus.Focus(delta: -1));

      final drawer = await boot(
        copyAppSettings(defaultAppSettings(), libraryDrawerEnabled: true),
      );
      drawer.read(libraryEvtDispatchProvider)(press);
      expect(drawer.read(libraryDrawerOpenProvider), isTrue);
      // The step itself still lands on the other pane.
      expect(drawer.read(libraryFocusZoneProvider), LibraryFocusZone.sidebar);

      // SHIFT + press dismisses it, and does not also move the focus cursor.
      drawer.read(libraryEvtDispatchProvider)(shiftPress);
      expect(drawer.read(libraryDrawerOpenProvider), isFalse);
      expect(drawer.read(libraryFocusZoneProvider), LibraryFocusZone.sidebar);

      // Docked mode has nothing to open or dismiss: both directions step focus.
      final docked = await boot(defaultAppSettings());
      docked.read(libraryEvtDispatchProvider)(shiftPress);
      expect(docked.read(libraryDrawerOpenProvider), isFalse);
      expect(docked.read(libraryFocusZoneProvider), LibraryFocusZone.sidebar);
    },
  );

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
    // Narrowness is not what hides metadata, room is: the metadata run keeps a
    // floor before the trailing cluster takes anything, so the title keeps its
    // artist and the chips keep the saved order. Album and genre fit the single
    // run; the length, last in the default order, is the one that cannot.
    expect(find.text('Artist'), findsOneWidget);
    expect(find.text('Album X'), findsOneWidget);
    expect(find.text('Genre Y'), findsOneWidget);
    expect(find.text('3:00'), findsNothing);
    expect(find.byIcon(LucideIcons.arrowLeft), findsOneWidget);
    // Its hit/hover box matches the row controls beside it (the density
    // button), so the wash fills the header instead of sitting as a small chip.
    expect(
      tester.getSize(find.byType(LibraryPaneBackButton)).height,
      tester.getSize(find.byType(LibraryRowDensityButton)).height,
    );

    await tester.tap(find.byIcon(LucideIcons.arrowLeft));
    await tester.pumpAndSettle();

    expect(find.byKey(kMTabBarKey), findsOneWidget);
    expect(find.byKey(kLibraryToolbarKey), findsNothing);
    expect(find.byIcon(LucideIcons.arrowLeft), findsNothing);
  });

  testWidgets('a narrow page with room keeps the full metadata run', (
    tester,
  ) async {
    // Still the single-page drawer layout (under the split threshold), but wide
    // enough for the row: nothing is hidden because the layout is narrow.
    await pumpSized(
      tester,
      size: const Size(900, 600),
      child: const SizedBox(width: 900, height: 600, child: LibraryPanel()),
    );

    await tester.tap(find.text('samples'));
    await tester.pumpAndSettle();

    expect(find.byKey(kMTabBarKey), findsNothing);
    expect(find.text('Demo Track'), findsOneWidget);
    expect(find.text('Artist'), findsOneWidget);
    expect(find.text('Album X'), findsOneWidget);
    expect(find.text('Genre Y'), findsOneWidget);
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
    // Split mode has the room, so the full metadata run stays.
    expect(find.text('Album X'), findsOneWidget);
    expect(find.text('Genre Y'), findsOneWidget);
    expect(find.byIcon(LucideIcons.arrowLeft), findsNothing);
  });

  testWidgets('drawer mode hides the docked panel and opens flush', (
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

    final container = mixerContainer(tester);
    container.read(libraryDrawerOpenProvider.notifier).set(true);
    await tester.pumpAndSettle();

    // A 480px drawer is narrower than the split threshold, so the list page
    // shows. It is flush to its edge.
    expect(find.byType(LibraryPanel), findsOneWidget);
    // Scoped to the drawer: the mixer's own tabs share the key.
    expect(
      find.descendant(
        of: find.byType(LibraryPanel),
        matching: find.byKey(kMTabBarKey),
      ),
      findsOneWidget,
    );
    expect(find.byKey(kLibraryToolbarKey), findsNothing);
    expect(tester.getTopLeft(find.byType(LibraryPanel)).dx, 0);
  });

  testWidgets('the drawer rules its inner edge over the panel', (tester) async {
    // A border painted under the panel is invisible: the panel draws its own
    // opaque surface on top. In the foreground it reads like the edit drawer.
    for (final side in LibraryDrawerSideSetting.values) {
      await pumpSized(
        tester,
        size: const Size(1200, 800),
        settings: copyAppSettings(
          defaultAppSettings(),
          libraryDrawerEnabled: true,
          libraryDrawerSide: side,
        ),
        child: const MixerPage(),
      );

      final container = mixerContainer(tester);
      container.read(libraryDrawerOpenProvider.notifier).set(true);
      await tester.pumpAndSettle();

      final rule = tester
          .widgetList<DecoratedBox>(
            find.ancestor(
              of: find.byType(LibraryPanel),
              matching: find.byType(DecoratedBox),
            ),
          )
          .firstWhere((box) => box.position == DecorationPosition.foreground);
      final border = (rule.decoration as BoxDecoration).border! as Border;
      final facing = switch (side) {
        LibraryDrawerSideSetting.left => border.right,
        LibraryDrawerSideSetting.right => border.left,
        LibraryDrawerSideSetting.top => border.bottom,
        LibraryDrawerSideSetting.bottom => border.top,
      };

      expect(facing.width, greaterThan(0), reason: '$side has no inner rule');
      expect(
        [
          border.left,
          border.top,
          border.right,
          border.bottom,
        ].where((edge) => edge.width > 0).length,
        1,
        reason: '$side rules more than its inner edge',
      );

      // Tear down between sides: `pumpWidget` reuses the `ProviderScope` and
      // its container, so the next side's settings would be ignored.
      await tester.pumpWidget(const SizedBox.shrink());
    }
  });

  testWidgets('a top drawer never paints over the titlebar', (tester) async {
    await pumpSized(
      tester,
      size: const Size(1200, 800),
      settings: copyAppSettings(
        defaultAppSettings(),
        libraryDrawerEnabled: true,
        libraryDrawerSide: LibraryDrawerSideSetting.top,
      ),
      child: const MixerPage(),
    );

    final container = mixerContainer(tester);
    container.read(libraryDrawerOpenProvider.notifier).set(true);
    // Mid-flight: the panel is still travelling through the header band.
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 100));

    final clipFinder = find
        .ancestor(
          of: find.byType(LibraryPanel),
          matching: find.byType(ClipRect),
        )
        .first;
    final clip = tester.widget<ClipRect>(clipFinder);
    final stage = tester.getRect(clipFinder);

    // The stage below the header, clipped hard, so the panel cannot paint over
    // the titlebar while it slides.
    expect(clip.clipBehavior, Clip.hardEdge);
    expect(stage.top, kAppHeaderHeight);
    expect(stage.height, 800 - kAppHeaderHeight);
    // This frame really is mid-slide: the panel still sits above the stage.
    expect(
      tester.getTopLeft(find.byType(LibraryPanel)).dy,
      lessThan(kAppHeaderHeight),
    );
  });

  testWidgets('a track drag tucks the drawer and its end closes it', (
    tester,
  ) async {
    await pumpSized(
      tester,
      size: const Size(1200, 800),
      settings: copyAppSettings(
        defaultAppSettings(),
        libraryDrawerEnabled: true,
      ),
      // The row only grows a drag source while the engine runs, which is also
      // the only state a track drag can start in.
      extra: [engineRunningProvider.overrideWithValue(true)],
      child: const MixerPage(),
    );

    final container = mixerContainer(tester);
    container.read(libraryDrawerOpenProvider.notifier).set(true);
    await tester.pumpAndSettle();

    // The default bottom drawer spans the width here, so the list shows.
    expect(find.byType(LibraryPanel), findsOneWidget);
    expect(find.text('Demo Track'), findsOneWidget);

    // Drag the row for real: the drag source is a Flutter `Draggable`, so a
    // pointer gesture drives it the way it drives the app.
    final gesture = await tester.startGesture(
      tester.getCenter(find.text('Demo Track')),
    );
    await gesture.moveBy(const Offset(40, 0));
    await tester.pumpAndSettle();

    // Tucked fully off its edge, so the deck it is dragged to is uncovered —
    // but still mounted: the row being dragged lives inside it, and unmounting
    // here is what aborted the native drag.
    expect(container.read(libraryDrawerOpenProvider), isTrue);
    expect(find.byType(LibraryPanel), findsOneWidget);
    expect(
      tester.getRect(find.byType(LibraryPanel)).top,
      greaterThanOrEqualTo(800),
    );

    // The drop lands: the drawer closes, and nothing reopens it. Step through
    // the close frame by frame — the panel is already off its edge, and the
    // close must not drag it back into view on the way out. It used to: the
    // tuck was released while the route still animated out, and the route's
    // window-height slide outran the panel-height un-tuck mid-animation.
    await gesture.up();
    for (var i = 0; i < 8; i++) {
      await tester.pump(const Duration(milliseconds: 40));
      final panels = find.byType(LibraryPanel);
      if (panels.evaluate().isEmpty) {
        break;
      }
      expect(
        tester.getRect(panels).top,
        greaterThanOrEqualTo(800),
        reason: 'the drawer must not appear while it closes',
      );
    }
    await tester.pumpAndSettle();
    expect(container.read(libraryDrawerOpenProvider), isFalse);
    expect(find.byType(LibraryPanel), findsNothing);
    // The tuck is deliberately held until the next open: releasing it while the
    // route closed is what pulled the panel back into view.
    expect(container.read(libraryTrackDragActiveProvider), isTrue);

    container.read(libraryDrawerOpenProvider.notifier).set(true);
    await tester.pumpAndSettle();
    expect(container.read(libraryTrackDragActiveProvider), isFalse);
    // Open in its normal place, not tucked off the edge.
    expect(tester.getRect(find.byType(LibraryPanel)).top, 320);
  });

  testWidgets('the header button closes the drawer it opened', (tester) async {
    await pumpSized(
      tester,
      size: const Size(1200, 800),
      settings: copyAppSettings(
        defaultAppSettings(),
        libraryDrawerEnabled: true,
      ),
      child: Column(
        children: [
          AppHeader(
            appTitle: 'Mixar',
            tab: ShellTab.mixer,
            onTabChanged: (_) {},
          ),
          const Expanded(child: MixerPage()),
        ],
      ),
    );

    final container = mixerContainer(tester);
    final button = find.descendant(
      of: find.byType(AppHeader),
      matching: find.byIcon(LucideIcons.libraryBig),
    );

    await tester.tap(button);
    await tester.pumpAndSettle();
    expect(find.byType(LibraryPanel), findsOneWidget);

    // The drawer's route has no barrier, so the header stays reachable and the
    // same button closes it. That reachability is also what lets a drop land on
    // a deck while the drawer is open.
    await tester.tap(button);
    await tester.pumpAndSettle();
    expect(find.byType(LibraryPanel), findsNothing);
    expect(container.read(libraryDrawerOpenProvider), isFalse);

    // One press reopens it.
    await tester.tap(button);
    await tester.pumpAndSettle();
    expect(find.byType(LibraryPanel), findsOneWidget);
  });

  testWidgets('the drawer stays below the app header', (tester) async {
    await pumpSized(
      tester,
      size: const Size(1200, 800),
      settings: copyAppSettings(
        defaultAppSettings(),
        libraryDrawerEnabled: true,
        libraryDrawerSide: LibraryDrawerSideSetting.right,
      ),
      child: const MixerPage(),
    );

    final container = mixerContainer(tester);
    container.read(libraryDrawerOpenProvider.notifier).set(true);
    await tester.pumpAndSettle();

    // The header holds the window controls, so a panel on any edge — the right
    // one included — has to start below it.
    final panel = tester.getRect(find.byType(LibraryPanel));
    expect(panel.top, kAppHeaderHeight);
    expect(panel.right, 1200);
    expect(panel.bottom, 800);
  });

  testWidgets('a press outside the drawer closes it, inside it works', (
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

    final container = mixerContainer(tester);
    container.read(libraryDrawerOpenProvider.notifier).set(true);
    await tester.pumpAndSettle();
    expect(find.byType(LibraryPanel), findsOneWidget);

    // A press inside the panel reaches it through the dismissal listener: the
    // 480px-wide drawer pages to the track list.
    await tester.tapAt(tester.getCenter(find.text('samples')));
    await tester.pumpAndSettle();
    expect(find.text('Demo Track'), findsOneWidget);
    expect(find.byType(LibraryPanel), findsOneWidget);

    // A press over the mixer, outside the panel, closes it.
    await tester.tapAt(const Offset(900, 400));
    await tester.pumpAndSettle();
    expect(find.byType(LibraryPanel), findsNothing);
    expect(container.read(libraryDrawerOpenProvider), isFalse);
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

  testWidgets('the drawer resizes from its inner edge and clamps', (
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

    final container = mixerContainer(tester);
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

    final container = mixerContainer(tester);
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
  testWidgets('a re-open during the closing animation stays open', (
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
    final container = mixerContainer(tester);

    container.read(libraryDrawerOpenProvider.notifier).set(true);
    await tester.pumpAndSettle();
    expect(find.byType(LibraryPanel), findsOneWidget);

    // Close, then open again while the 250 ms pop is still running: when the
    // closing route finishes it must not clear the flag the re-open owns.
    container.read(libraryDrawerOpenProvider.notifier).set(false);
    await tester.pump(const Duration(milliseconds: 100));
    container.read(libraryDrawerOpenProvider.notifier).set(true);
    await tester.pumpAndSettle();

    expect(container.read(libraryDrawerOpenProvider), isTrue);
    expect(
      find.byType(LibraryPanel),
      findsOneWidget,
      reason: 'the closing route shut the drawer the user just reopened',
    );
  });

  testWidgets('a drag source reports one start and one end per drag', (
    tester,
  ) async {
    var starts = 0;
    var ends = 0;
    await tester.pumpWidget(
      // The theme sits above the app, like it does in the real app: the drag
      // card is built in the overlay, which is inside the theme scope.
      MixarTheme(
        data: MixarThemeData.light(),
        child: MaterialApp(
          home: Center(
            child: libraryTrackDragSource(
              payload: const TrackDragPayload(
                source: TrackDragSource.library,
                path: '/tmp/samples/demo.wav',
                title: 'Demo Track',
              ),
              onDragStart: () => starts++,
              onDragEnd: () => ends++,
              // A coloured box, not a bare `SizedBox`: an empty box is not hit
              // testable, so the drag source would never see the pointer.
              child: const ColoredBox(
                color: Color(0xFF000000),
                child: SizedBox(width: 24, height: 24),
              ),
            ),
          ),
        ),
      ),
    );

    final gesture = await tester.startGesture(
      tester.getCenter(find.byType(SizedBox)),
    );
    await gesture.moveBy(const Offset(40, 0));
    await tester.pump();
    await gesture.up();
    await tester.pump();

    expect(starts, 1, reason: 'the drag starts once');
    expect(ends, 1, reason: 'the drag ends once');
  });
}
