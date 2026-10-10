import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/shell/material_theme.dart';
import 'package:gui_flutter/shell/mixar_menu.dart';
import 'package:gui_flutter/shell/mixar_overlay_controller.dart';
import 'package:gui_flutter/shell/mixar_popover.dart';
import 'package:gui_flutter/shell/mixar_theme.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';
import 'package:material_ui/material_ui.dart';

import 'support/mixar_material_app.dart';

void main() {
  testWidgets('MixarPopover toggles overlay content', (tester) async {
    final theme = MixarThemeData.dark();
    await tester.pumpWidget(
      MaterialApp(
        theme: materialUiThemeFromMixar(theme),
        builder: mixarMaterialAppBuilder(theme),
        home: Scaffold(
          body: MixarPopover(
            overlayBuilder: (context) => const Text('Gain panel'),
            childBuilder: (context, controller) => GestureDetector(
              onTap: controller.toggle,
              child: const Text('Open'),
            ),
          ),
        ),
      ),
    );

    expect(find.text('Gain panel'), findsNothing);
    await tester.tap(find.text('Open'));
    await tester.pumpAndSettle();
    expect(find.text('Gain panel'), findsOneWidget);

    await tester.tapAt(const Offset(300, 300));
    await tester.pumpAndSettle();
    expect(find.text('Gain panel'), findsNothing);
  });

  testWidgets('MixarMenuPanel keeps a fixed width', (tester) async {
    final theme = MixarThemeData.dark();
    await tester.pumpWidget(
      MaterialApp(
        theme: materialUiThemeFromMixar(theme),
        builder: mixarMaterialAppBuilder(theme),
        home: const Scaffold(
          body: Center(
            child: MixarMenuPanel(minWidth: 200, child: Text('Analyze')),
          ),
        ),
      ),
    );

    final panel = find.descendant(
      of: find.byType(MixarMenuPanel),
      matching: find.byWidgetPredicate((w) => w is SizedBox && w.width == 200),
    );
    expect(panel, findsOneWidget);
    expect(tester.getSize(panel).width, 200);
  });

  testWidgets('MixarMenuPanel clips row fills to its rounded border', (
    tester,
  ) async {
    // A full-bleed hover fill must not square off the panel's rounded corner.
    await tester.pumpWidget(
      MaterialApp(
        theme: materialUiThemeFromMixar(MixarThemeData.dark()),
        builder: mixarMaterialAppBuilder(MixarThemeData.dark()),
        home: const Scaffold(
          body: Center(child: MixarMenuPanel(child: Text('Analyze'))),
        ),
      ),
    );

    final clip = tester.widget<ClipRRect>(
      find.descendant(
        of: find.byType(MixarMenuPanel),
        matching: find.byType(ClipRRect),
      ),
    );
    expect(clip.borderRadius, MixarThemeData.dark().style.borderRadius.md);
  });

  testWidgets('MixarMenuAnchor dismisses on outside tap', (tester) async {
    final theme = MixarThemeData.dark();
    await tester.pumpWidget(
      MaterialApp(
        theme: materialUiThemeFromMixar(theme),
        builder: mixarMaterialAppBuilder(theme),
        home: Scaffold(
          body: SizedBox(
            width: 400,
            height: 400,
            child: Stack(
              children: [
                const Positioned.fill(
                  child: ColoredBox(color: Color(0xFF111111)),
                ),
                Align(
                  alignment: Alignment.topLeft,
                  child: MixarMenuAnchor(
                    style: SelectStyleSetting.desktop,
                    menuBuilder: (context, controller) =>
                        const Text('Menu body'),
                    childBuilder: (context, controller) => GestureDetector(
                      onTap: controller.toggle,
                      child: const Text('Open menu'),
                    ),
                  ),
                ),
              ],
            ),
          ),
        ),
      ),
    );

    await tester.tap(find.text('Open menu'));
    await tester.pumpAndSettle();
    expect(find.text('Menu body'), findsOneWidget);

    await tester.tapAt(const Offset(350, 350));
    await tester.pumpAndSettle();
    expect(find.text('Menu body'), findsNothing);
  });

  testWidgets('MixarMenuAnchor mobile style opens a dialog', (tester) async {
    final theme = MixarThemeData.dark();
    await tester.pumpWidget(
      MaterialApp(
        theme: materialUiThemeFromMixar(theme),
        builder: mixarMaterialAppBuilder(theme),
        home: Scaffold(
          body: Center(
            child: MixarMenuAnchor(
              style: SelectStyleSetting.mobile,
              menuBuilder: (context, controller) => MixarMenuBody(
                groups: [
                  MixarMenuGroup(
                    children: [
                      MixarMenuItem(
                        title: const Text('Analyze tracks…'),
                        onPress: controller.hide,
                      ),
                    ],
                  ),
                ],
              ),
              childBuilder: (context, controller) => GestureDetector(
                onTap: controller.toggle,
                child: const Text('Open menu'),
              ),
            ),
          ),
        ),
      ),
    );

    await tester.tap(find.text('Open menu'));
    await tester.pumpAndSettle();
    expect(find.text('Analyze tracks…'), findsOneWidget);

    await tester.tap(find.text('Analyze tracks…'));
    await tester.pumpAndSettle();
    expect(find.text('Analyze tracks…'), findsNothing);
  });

  testWidgets(
    'MixarMenuAnchor mobile barrier dismiss clears isShowing',
    (tester) async {
      final theme = MixarThemeData.dark();
      late MixarOverlayController menuController;
      await tester.pumpWidget(
        MaterialApp(
          theme: materialUiThemeFromMixar(theme),
          builder: mixarMaterialAppBuilder(theme),
          home: Scaffold(
            body: Center(
              child: MixarMenuAnchor(
                style: SelectStyleSetting.mobile,
                menuBuilder: (context, controller) => MixarMenuBody(
                  groups: [
                    MixarMenuGroup(
                      children: [
                        MixarMenuItem(
                          title: const Text('Analyze tracks…'),
                          onPress: controller.hide,
                        ),
                      ],
                    ),
                  ],
                ),
                childBuilder: (context, controller) {
                  menuController = controller;
                  return GestureDetector(
                    onTap: controller.toggle,
                    child: const Text('Open menu'),
                  );
                },
              ),
            ),
          ),
        ),
      );

      await tester.tap(find.text('Open menu'));
      await tester.pumpAndSettle();
      expect(find.text('Analyze tracks…'), findsOneWidget);
      expect(menuController.isShowing, isTrue);

      // Barrier dismiss (same corner tap as mixar_dialog_test).
      await tester.tapAt(const Offset(8, 8));
      await tester.pumpAndSettle();
      expect(find.text('Analyze tracks…'), findsNothing);
      expect(menuController.isShowing, isFalse);
    },
  );
}
