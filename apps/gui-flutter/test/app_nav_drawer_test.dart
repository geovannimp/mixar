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
import 'package:gui_flutter/widgets/shell/app_nav_drawer.dart';
import 'package:gui_flutter/widgets/shell/m_tappable.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:gui_flutter/widgets/shell/nav_list_item.dart';
import 'package:gui_flutter/widgets/shell/window_title_bar_controls.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';
import 'package:material_ui/material_ui.dart';
import 'package:window_manager/window_manager.dart';

import 'helpers/collection_tracks_override.dart';
import 'support/mixar_material_app.dart';

const _windowManagerChannel = MethodChannel('window_manager');

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

  Future<void> openMenu(WidgetTester tester) async {
    await tester.tap(
      find.descendant(
        of: find.byType(AppHeader),
        matching: find.byIcon(LucideIcons.menu),
      ),
    );
    await tester.pumpAndSettle();
  }

  /// Answers window_manager on a desktop host and records what the header
  /// asked it, so a test can prove the app's own buttons were never built.
  List<String> recordWindowCalls() {
    final calls = <String>[];
    final messenger =
        TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
          ..setMockMethodCallHandler(_windowManagerChannel, (call) async {
            calls.add(call.method);
            // The window's buttons read these two; the rest is unused.
            return switch (call.method) {
              'isMaximized' || 'isMinimized' => false,
              _ => null,
            };
          });
    addTearDown(
      () => messenger.setMockMethodCallHandler(_windowManagerChannel, null),
    );
    return calls;
  }

  /// The menu button's leading edge — the header's first content, so its x is
  /// the corner the window controls own.
  Finder menuButton() => find.descendant(
    of: find.byType(AppHeader),
    matching: find.byWidgetPredicate(
      (widget) => widget is AppButton && widget.semanticsLabel == 'Menu',
    ),
  );

  double menuLeft(WidgetTester tester) => tester.getTopLeft(menuButton()).dx;

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

  /// Pumps the header off-desktop and returns the menu button's x — the
  /// baseline each host case measures its leading inset against.
  Future<double> pumpHeaderBaseline(
    WidgetTester tester, {
    MixarThemeData? theme,
    Size surfaceSize = const Size(900, 600),
  }) async {
    await pumpHeader(tester, theme: theme, surfaceSize: surfaceSize);
    return menuLeft(tester);
  }

  /// A drawer row, addressed by the shared widget's own label rather than by
  /// rendered text.
  Finder rowFinder(String label) =>
      find.byWidgetPredicate((w) => w is NavListItem && w.label == label);

  MTappable row(WidgetTester tester, String label) => tester.widget<MTappable>(
    find.descendant(of: rowFinder(label), matching: find.byType(MTappable)),
  );

  testWidgets('the hamburger lists the shell pages', (tester) async {
    await pumpHeader(tester);
    expect(rowFinder('Play'), findsNothing);

    await openMenu(tester);

    expect(find.byType(AppHeader), findsOneWidget);
    expect(rowFinder('Play'), findsOneWidget);
    expect(rowFinder('Settings'), findsOneWidget);
    // The wordmark stays in the titlebar and heads the menu.
    final wordmark = find.byWidgetPredicate(
      (w) => w is Image && w.semanticLabel == 'Mixar',
    );
    expect(wordmark, findsWidgets);
    final inMenu = wordmark.evaluate().where(
      (e) =>
          tester.getRect(find.byElementPredicate((w) => w == e)).top >=
          kAppHeaderHeight,
    );
    expect(inMenu, isNotEmpty, reason: 'the menu carries its own wordmark');

    // The panel is inset below the header, so it never covers the chrome that
    // opened it.
    expect(
      tester.getTopLeft(rowFinder('Play')).dy,
      greaterThan(kAppHeaderHeight),
    );
    expect(
      tester.getRect(find.byType(AppHeader)).width,
      greaterThan(kAppNavDrawerWidth),
      reason: 'the panel is a drawer, not the whole window',
    );
    // Rows are full-bleed, so the row box *is* the panel width.
    for (final item in find.byType(NavListItem).evaluate()) {
      expect(
        tester.getSize(find.byElementPredicate((w) => w == item)).width,
        kAppNavDrawerWidth,
      );
    }
  });

  testWidgets('escape closes the drawer', (tester) async {
    await pumpHeader(tester);
    await openMenu(tester);
    expect(rowFinder('Play'), findsOneWidget);

    await tester.sendKeyEvent(LogicalKeyboardKey.escape);
    await tester.pumpAndSettle();

    expect(rowFinder('Play'), findsNothing);
  });

  testWidgets('the open page is marked in the drawer', (tester) async {
    await pumpHeader(tester, tab: ShellTab.settings);
    await openMenu(tester);

    expect(row(tester, 'Settings').selected, isTrue);
    expect(row(tester, 'Play').selected, isFalse);
  });

  testWidgets('picking a page reports it and closes the drawer', (
    tester,
  ) async {
    final picked = await pumpHeader(tester);
    await openMenu(tester);

    await tester.tap(rowFinder('Settings'));
    await tester.pumpAndSettle();

    expect(picked, [ShellTab.settings]);
    expect(rowFinder('Play'), findsNothing);
  });

  testWidgets('a press outside the drawer closes it without navigating', (
    tester,
  ) async {
    final picked = await pumpHeader(tester);
    await openMenu(tester);

    await tester.tapAt(const Offset(700, 400));
    await tester.pumpAndSettle();

    expect(picked, isEmpty);
    expect(rowFinder('Play'), findsNothing);
  });

  testWidgets('the header no longer carries a settings button', (tester) async {
    await pumpHeader(tester);
    expect(
      find.descendant(
        of: find.byType(AppHeader),
        matching: find.byIcon(LucideIcons.settings),
      ),
      findsNothing,
    );
  });

  testWidgets('the wordmark sits inside the menu button', (tester) async {
    await pumpHeader(tester);

    expect(
      find.descendant(
        of: find.byWidgetPredicate(
          (w) => w is AppButton && w.semanticsLabel == 'Menu',
        ),
        matching: find.byWidgetPredicate(
          (w) => w is Image && w.semanticLabel == 'Mixar',
        ),
      ),
      findsOneWidget,
      reason: 'the logo is part of the control that opens the menu',
    );
  });

  for (final theme in [MixarThemeData.dark(), MixarThemeData.light()]) {
    final mode = theme.colors.brightness.name;

    testWidgets('a macOS window keeps the corner for its traffic lights '
        '($mode)', (tester) async {
      final calls = recordWindowCalls();
      const surface = Size(1024, 768);

      // Baseline: not a desktop window, so nothing sits before the menu.
      final baseline = await pumpHeaderBaseline(
        tester,
        theme: theme,
        surfaceSize: surface,
      );

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
            menuLeft(tester),
            baseline + kMacOSWindowControlsInset,
            reason: 'the menu clears the system buttons by the same inset',
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
            reason: 'the empty middle still moves the window',
          );

          // The menu still opens, shifted header and all.
          await openMenu(tester);
          expect(rowFinder('Settings'), findsOneWidget);
          expect(tester.takeException(), isNull);
          expect(
            calls,
            isNot(contains('isMaximized')),
            reason: 'no app buttons were built, so nothing asked the window',
          );

          await unmount(tester);
        },
      );
    });

    for (final platform in [TargetPlatform.windows, TargetPlatform.linux]) {
      testWidgets('a ${platform.name} window keeps the app buttons ($mode)', (
        tester,
      ) async {
        final calls = recordWindowCalls();

        final baseline = await pumpHeaderBaseline(tester, theme: theme);

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
              menuLeft(tester),
              baseline,
              reason: 'no system buttons there, so nothing is reserved',
            );
            expect(find.byType(WindowTitleBarControls), findsOneWidget);
            expect(find.byType(WindowCaptionButton), findsNWidgets(3));
            expect(previewLights(), findsNothing);
            expect(
              tester.getSize(find.byType(AppHeader)).height,
              kAppHeaderHeight,
            );
            expect(calls, contains('isMaximized'));

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
      final calls = recordWindowCalls();

      final baseline = await pumpHeaderBaseline(tester, theme: theme);

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

          expect(menuLeft(tester), baseline + kMacOSWindowControlsInset);
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
          expect(calls, isNot(contains('isMaximized')));

          await unmount(tester);
        },
      );
    });
  }
}
