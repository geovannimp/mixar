import 'package:flutter/foundation.dart'
    show TargetPlatform, debugDefaultTargetPlatformOverride;
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/models/shell_tab.dart';
import 'package:gui_flutter/core/utils/desktop.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/providers/library/history_providers.dart';
import 'package:gui_flutter/providers/library/providers.dart';
import 'package:gui_flutter/providers/mixer/deck_load.dart';
import 'package:gui_flutter/providers/settings/settings_providers.dart';
import 'package:gui_flutter/widgets/shell/app_button.dart';
import 'package:gui_flutter/widgets/shell/app_header.dart';
import 'package:gui_flutter/widgets/shell/m_tappable.dart';
import 'package:gui_flutter/widgets/shell/mixar_menu_panel.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:gui_flutter/widgets/shell/nav_list_item.dart';
import 'package:gui_flutter/widgets/shell/window_title_bar_controls.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';
import 'package:material_ui/material_ui.dart';
import 'package:window_manager/window_manager.dart';

import 'helpers/collection_tracks_override.dart';
import 'support/mixar_material_app.dart';

void main() {
  /// Runs [body] as a [platform] host with the desktop-window and macOS
  /// preview overrides set, restoring all three before the framework's
  /// end-of-test invariant check — that check runs before `addTearDown`.
  Future<void> withHost(
    TargetPlatform? platform, {
    required bool desktop,
    required Future<void> Function() body,
    bool macOSPreview = false,
  }) async {
    debugOverrideDesktopWindow = desktop;
    debugOverrideMacOSChromePreview = macOSPreview;
    debugDefaultTargetPlatformOverride = platform;
    try {
      await body();
    } finally {
      debugOverrideDesktopWindow = null;
      debugOverrideMacOSChromePreview = null;
      debugDefaultTargetPlatformOverride = null;
    }
  }

  /// Pumps the header on [platform], with or without a desktop window, and
  /// with the macOS chrome preview on when [macOSPreview] asks for it.
  ///
  /// Every override and view value is restored after the test.
  Future<List<ShellTab>> pumpHeader(
    WidgetTester tester, {
    ShellTab tab = ShellTab.mixer,
    bool desktop = false,
    TargetPlatform? platform,
    MixarThemeData? theme,
    Size surfaceSize = const Size(900, 600),
    bool macOSPreview = false,
  }) async {
    debugOverrideDesktopWindow = desktop;
    debugOverrideMacOSChromePreview = macOSPreview;
    debugDefaultTargetPlatformOverride = platform;
    addTearDown(() {
      debugOverrideDesktopWindow = null;
      debugOverrideMacOSChromePreview = null;
      debugDefaultTargetPlatformOverride = null;
    });
    tester.view.physicalSize = surfaceSize;
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);

    final picked = <ShellTab>[];
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          collectionsProvider.overrideWith((ref) async => const []),
          overrideCollectionTracksValue(const []),
          historySessionsProvider.overrideWith((ref) async => const []),
          historyCanResumeProvider.overrideWith((ref) async => false),
          historySettingsBootstrapProvider.overrideWith((ref) {}),
          librarySettingsBootstrapProvider.overrideWith((ref) {}),
          focusedDeckLoadBootstrapProvider.overrideWith((ref) {}),
          appSettingsProvider.overrideWith((ref) async => defaultAppSettings()),
        ],
        child: mixarTestMaterialApp(
          theme: theme ?? MixarThemeData.dark(),
          home: Scaffold(
            body: Column(
              children: [
                AppHeader(
                  appTitle: 'Mixar',
                  tab: tab,
                  onTabChanged: (tab) async => picked.add(tab),
                ),
              ],
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();
    return picked;
  }

  /// The header's one navigation control: wordmark + open page.
  Finder pagesButton() => find.descendant(
    of: find.byType(AppHeader),
    matching: find.byWidgetPredicate(
      (widget) =>
          widget is AppButton &&
          (widget.semanticsLabel?.startsWith('Pages') ?? false),
    ),
  );

  Finder withinPagesButton(Finder matching) =>
      find.descendant(of: pagesButton(), matching: matching);

  Future<void> openMenu(WidgetTester tester) async {
    await tester.tap(pagesButton());
    await tester.pumpAndSettle();
  }

  /// A menu row, addressed by the shared widget's own label rather than by
  /// rendered text — the header shows the same words.
  Finder rowFinder(String label) =>
      find.byWidgetPredicate((w) => w is NavListItem && w.label == label);

  MTappable row(WidgetTester tester, String label) => tester.widget<MTappable>(
    find.descendant(of: rowFinder(label), matching: find.byType(MTappable)),
  );

  /// The preview's reference lights, if any were painted.
  Finder previewLights() => find.byWidgetPredicate(
    (widget) =>
        widget is Semantics &&
        widget.properties.label == 'macOS title-bar layout preview',
  );

  /// Drops the tree so window listeners let go before the mocks are cleared.
  Future<void> unmount(WidgetTester tester) async {
    await tester.pumpWidget(const SizedBox.shrink());
    await tester.pump();
  }

  testWidgets('the header names the open page beside the wordmark', (
    tester,
  ) async {
    await pumpHeader(tester);

    expect(
      withinPagesButton(
        find.byWidgetPredicate((w) => w is Image && w.semanticLabel == 'Mixar'),
      ),
      findsOneWidget,
      reason: 'the wordmark is part of the pages control',
    );
    expect(withinPagesButton(find.text('Booth')), findsOneWidget);

    await pumpHeader(tester, tab: ShellTab.settings);
    expect(withinPagesButton(find.text('Settings')), findsOneWidget);
    expect(withinPagesButton(find.text('Booth')), findsNothing);
  });

  testWidgets('the pages control sits at the centre of the window', (
    tester,
  ) async {
    // The header surface is 900 wide by default: the title lands on 450.
    await pumpHeader(tester);
    expect(
      tester.getRect(pagesButton()).center.dx,
      moreOrLessEquals(450, epsilon: 1),
      reason: 'the centred title control owns the middle',
    );

    // The trailing corner changes with the page (the Library toggle only
    // exists on the mixer); the title must not drift with it.
    await pumpHeader(tester, tab: ShellTab.settings);
    expect(
      tester.getRect(pagesButton()).center.dx,
      moreOrLessEquals(450, epsilon: 1),
    );
  });

  testWidgets('the hamburger is gone and the title control is the menu', (
    tester,
  ) async {
    await pumpHeader(tester);
    expect(
      find.descendant(
        of: find.byType(AppHeader),
        matching: find.byIcon(LucideIcons.menu),
      ),
      findsNothing,
    );
    expect(rowFinder('Booth'), findsNothing);
  });

  testWidgets('pressing the title lists every page', (tester) async {
    await pumpHeader(tester);
    await openMenu(tester);

    expect(rowFinder('Booth'), findsOneWidget);
    expect(rowFinder('Settings'), findsOneWidget);

    // The dropdown drops from the control, under the header.
    final panel = tester.getRect(find.byType(MixarMenuPanel));
    expect(panel.top, greaterThanOrEqualTo(kAppHeaderHeight));
    expect(
      panel.center.dx,
      moreOrLessEquals(tester.getRect(pagesButton()).center.dx, epsilon: 1),
      reason: 'the menu hangs centred under the title control',
    );
  });

  testWidgets('the open page is marked in the menu', (tester) async {
    await pumpHeader(tester, tab: ShellTab.settings);
    await openMenu(tester);

    expect(row(tester, 'Settings').selected, isTrue);
    expect(row(tester, 'Booth').selected, isFalse);
  });

  testWidgets('picking a page reports it and closes the menu', (tester) async {
    final picked = await pumpHeader(tester);
    await openMenu(tester);

    await tester.tap(rowFinder('Settings'));
    await tester.pumpAndSettle();

    expect(picked, [ShellTab.settings]);
    expect(rowFinder('Booth'), findsNothing);
  });

  testWidgets('a press outside the menu closes it without navigating', (
    tester,
  ) async {
    final picked = await pumpHeader(tester);
    await openMenu(tester);

    await tester.tapAt(const Offset(700, 500));
    await tester.pumpAndSettle();

    expect(picked, isEmpty);
    expect(rowFinder('Booth'), findsNothing);
  });

  testWidgets('escape closes the menu', (tester) async {
    await pumpHeader(tester);
    await openMenu(tester);
    expect(rowFinder('Booth'), findsOneWidget);

    await tester.sendKeyEvent(LogicalKeyboardKey.escape);
    await tester.pumpAndSettle();

    expect(rowFinder('Booth'), findsNothing);
  });

  for (final theme in [MixarThemeData.dark(), MixarThemeData.light()]) {
    final mode = theme.colors.brightness.name;

    testWidgets('a macOS window keeps the middle and its lights ($mode)', (
      tester,
    ) async {
      const surface = Size(1024, 768);

      await withHost(
        TargetPlatform.macOS,
        desktop: true,
        body: () async {
          await pumpHeader(
            tester,
            desktop: true,
            platform: TargetPlatform.macOS,
            theme: theme,
            surfaceSize: surface,
          );

          expect(
            tester.getRect(pagesButton()).center.dx,
            moreOrLessEquals(surface.width / 2, epsilon: 1),
          );
          expect(find.byType(WindowTitleBarControls), findsNothing);
          expect(find.byType(WindowCaptionButton), findsNothing);
          expect(
            previewLights(),
            findsNothing,
            reason: 'a Mac draws real controls, never the preview markers',
          );
          for (final label in ['Close', 'Minimize', 'Zoom']) {
            expect(
              find.byWidgetPredicate(
                (widget) =>
                    widget is MTappable && widget.semanticsLabel == label,
              ),
              findsOneWidget,
              reason: 'the header owns the $label light',
            );
          }
          expect(
            tester.getSize(find.byType(AppHeader)).height,
            kAppHeaderHeight,
          );
          expect(
            find.byType(DragToMoveArea),
            findsOneWidget,
            reason: 'the empty chrome still moves the window',
          );

          await openMenu(tester);
          expect(rowFinder('Settings'), findsOneWidget);
          expect(tester.takeException(), isNull);

          await unmount(tester);
        },
      );
    });

    for (final platform in [TargetPlatform.windows, TargetPlatform.linux]) {
      testWidgets('a ${platform.name} window keeps the app buttons ($mode)', (
        tester,
      ) async {
        await withHost(
          platform,
          desktop: true,
          body: () async {
            await pumpHeader(
              tester,
              desktop: true,
              platform: platform,
              theme: theme,
              surfaceSize: const Size(1024, 768),
            );

            expect(
              tester.getRect(pagesButton()).center.dx,
              moreOrLessEquals(512, epsilon: 1),
            );
            expect(find.byType(WindowTitleBarControls), findsOneWidget);
            expect(find.byType(WindowCaptionButton), findsNWidgets(3));
            expect(previewLights(), findsNothing);
            expect(
              tester.getSize(find.byType(AppHeader)).height,
              kAppHeaderHeight,
            );
            expect(
              tester.getRect(pagesButton()).right,
              lessThan(
                tester.getRect(find.byType(WindowTitleBarControls)).left,
              ),
              reason: 'the title never runs under the window buttons',
            );

            await openMenu(tester);
            expect(rowFinder('Settings'), findsOneWidget);
            expect(tester.takeException(), isNull);

            await unmount(tester);
          },
        );
      });
    }

    testWidgets('the Linux preview draws the reference lights ($mode)', (
      tester,
    ) async {
      await withHost(
        TargetPlatform.linux,
        desktop: true,
        macOSPreview: true,
        body: () async {
          await pumpHeader(
            tester,
            desktop: true,
            platform: TargetPlatform.linux,
            macOSPreview: true,
            theme: theme,
            surfaceSize: const Size(1024, 768),
          );

          expect(
            tester.getRect(pagesButton()).center.dx,
            moreOrLessEquals(512, epsilon: 1),
          );
          expect(find.byType(WindowTitleBarControls), findsNothing);
          expect(find.byType(WindowCaptionButton), findsNothing);
          expect(previewLights(), findsOneWidget);
          expect(
            tester.getSize(find.byType(AppHeader)).height,
            kAppHeaderHeight,
          );

          await openMenu(tester);
          expect(rowFinder('Settings'), findsOneWidget);
          expect(tester.takeException(), isNull);

          await unmount(tester);
        },
      );
    });
  }
}
