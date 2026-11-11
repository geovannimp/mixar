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
import 'package:gui_flutter/widgets/shell/nav_list_item.dart';
import 'package:gui_flutter/widgets/shell/m_tappable.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';
import 'package:material_ui/material_ui.dart';

import 'helpers/collection_tracks_override.dart';
import 'support/mixar_material_app.dart';

void main() {
  Future<List<ShellTab>> pumpHeader(
    WidgetTester tester, {
    ShellTab tab = ShellTab.mixer,
  }) async {
    debugOverrideDesktopWindow = false;
    addTearDown(() => debugOverrideDesktopWindow = null);
    tester.view.physicalSize = const Size(900, 600);
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
          theme: MixarThemeData.dark(),
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

  MTappable row(WidgetTester tester, String label) => tester.widget<MTappable>(
    find.byWidgetPredicate((w) => w is MTappable && w.semanticsLabel == label),
  );

  testWidgets('the hamburger lists the shell pages', (tester) async {
    await pumpHeader(tester);
    expect(find.text('Play'), findsNothing);

    await openMenu(tester);

    expect(find.byType(AppHeader), findsOneWidget);
    expect(
      find.descendant(of: find.byType(MTappable), matching: find.text('Play')),
      findsOneWidget,
    );
    expect(
      find.descendant(
        of: find.byType(MTappable),
        matching: find.text('Settings'),
      ),
      findsOneWidget,
    );
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
      tester.getTopLeft(find.text('Play')).dy,
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
    expect(find.text('Play'), findsOneWidget);

    await tester.sendKeyEvent(LogicalKeyboardKey.escape);
    await tester.pumpAndSettle();

    expect(find.text('Play'), findsNothing);
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

    await tester.tap(find.text('Settings'));
    await tester.pumpAndSettle();

    expect(picked, [ShellTab.settings]);
    expect(find.text('Play'), findsNothing);
  });

  testWidgets('a press outside the drawer closes it without navigating', (
    tester,
  ) async {
    final picked = await pumpHeader(tester);
    await openMenu(tester);

    await tester.tapAt(const Offset(700, 400));
    await tester.pumpAndSettle();

    expect(picked, isEmpty);
    expect(find.text('Play'), findsNothing);
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
}
