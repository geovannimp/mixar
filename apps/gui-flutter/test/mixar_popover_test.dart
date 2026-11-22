import 'dart:math' as math;

import 'package:flutter/gestures.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/theme/material_theme.dart';
import 'package:gui_flutter/providers/shell/mixar_overlay_controller.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';
import 'package:gui_flutter/widgets/shell/app_header.dart';
import 'package:gui_flutter/widgets/shell/m_divider.dart';
import 'package:gui_flutter/widgets/shell/mixar_menu_anchor.dart';
import 'package:gui_flutter/widgets/shell/mixar_menu_body.dart';
import 'package:gui_flutter/widgets/shell/mixar_menu_group.dart';
import 'package:gui_flutter/widgets/shell/mixar_menu_item.dart';
import 'package:gui_flutter/widgets/shell/mixar_menu_panel.dart';
import 'package:gui_flutter/widgets/shell/mixar_popover.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:material_ui/material_ui.dart';

import 'support/mixar_material_app.dart';

void main() {
  Future<MixarOverlayController?> pumpMenuAnchor(
    WidgetTester tester, {
    SelectStyleSetting style = SelectStyleSetting.mobile,
    bool captureController = false,
    double minWidth = 200,
    String? dialogTitle,
  }) async {
    final theme = MixarThemeData.dark();
    MixarOverlayController? menuController;
    await tester.pumpWidget(
      MaterialApp(
        theme: materialUiThemeFromMixar(theme),
        builder: mixarMaterialAppBuilder(theme),
        home: Scaffold(
          body: Center(
            child: MixarMenuAnchor(
              style: style,
              minWidth: minWidth,
              dialogTitle: dialogTitle,
              menuBuilder: (context, controller) => MixarMenuBody(
                groups: [
                  MixarMenuGroup(
                    children: [
                      MixarMenuItem(
                        title: const Text('Analyze tracks…'),
                        onPress: controller.hide,
                      ),
                      MixarMenuItem(
                        title: const Text('Refresh'),
                        onPress: controller.hide,
                      ),
                    ],
                  ),
                ],
              ),
              childBuilder: (context, controller) {
                if (captureController) {
                  menuController = controller;
                }
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
    return menuController;
  }

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

  testWidgets('MixarMenuPanel clips row fills to its border radius', (
    tester,
  ) async {
    // A full-bleed hover fill must not bleed past the panel outline.
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
    expect(clip.borderRadius, MixarThemeData.dark().style.borderRadius.dialog);

    // Rows paint edge to edge, so the stroke must sit above them.
    final foregroundStroke = tester
        .widgetList<DecoratedBox>(
          find.descendant(
            of: find.byType(MixarMenuPanel),
            matching: find.byType(DecoratedBox),
          ),
        )
        .where(
          (w) =>
              w.position == DecorationPosition.foreground &&
              (w.decoration as BoxDecoration).border != null,
        );
    expect(foregroundStroke, hasLength(1));
  });

  /// Pumps a 30-row menu whose trigger sits at [alignment] in an 800×400
  /// window, opens it, and returns the picked row.
  Future<int Function()> pumpLongMenu(
    WidgetTester tester, {
    required Alignment alignment,
  }) async {
    tester.view.physicalSize = const Size(800, 400);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);

    final theme = MixarThemeData.dark();
    var picked = 0;
    await tester.pumpWidget(
      MaterialApp(
        theme: materialUiThemeFromMixar(theme),
        builder: mixarMaterialAppBuilder(theme),
        home: Scaffold(
          body: Align(
            alignment: alignment,
            child: MixarMenuAnchor(
              style: SelectStyleSetting.desktop,
              menuBuilder: (context, controller) => MixarMenuBody(
                groups: [
                  MixarMenuGroup(
                    children: [
                      for (var i = 1; i <= 30; i++)
                        MixarMenuItem(
                          title: Text('Row $i'),
                          onPress: () {
                            picked = i;
                            controller.hide();
                          },
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
    return () => picked;
  }

  testWidgets('a long menu scrolls inside the room beside its trigger', (
    tester,
  ) async {
    final picked = await pumpLongMenu(tester, alignment: Alignment.center);

    // Neither side of a mid-window trigger holds 30 rows: the panel is bounded
    // by that room, not by the whole window, and the last row stays reachable.
    final trigger = tester.getRect(find.text('Open menu'));
    final room =
        math.max(trigger.top, 400 - trigger.bottom) - kMixarMenuEdgeMargin;
    expect(
      tester.getRect(find.byType(MixarMenuPanel)).height,
      lessThanOrEqualTo(room + 1),
    );

    await tester.scrollUntilVisible(
      find.text('Row 30'),
      200,
      scrollable: find.byType(Scrollable),
    );
    await tester.tap(find.text('Row 30'));
    await tester.pumpAndSettle();

    expect(picked(), 30);
    expect(find.text('Row 30'), findsNothing);
  });

  testWidgets('a menu near the bottom edge opens upward and fits', (
    tester,
  ) async {
    await pumpLongMenu(tester, alignment: Alignment.bottomLeft);

    final panel = tester.getRect(find.byType(MixarMenuPanel));
    final trigger = tester.getRect(find.text('Open menu'));
    expect(panel.bottom, lessThanOrEqualTo(trigger.top));
    expect(panel.top, greaterThanOrEqualTo(0));

    await tester.scrollUntilVisible(
      find.text('Row 30'),
      200,
      scrollable: find.byType(Scrollable),
    );
    expect(find.text('Row 30'), findsOneWidget);
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
    await pumpMenuAnchor(tester, dialogTitle: 'Collection actions');

    await tester.tap(find.text('Open menu'));
    await tester.pumpAndSettle();
    expect(find.text('Collection actions'), findsOneWidget);
    expect(find.text('Analyze tracks…'), findsOneWidget);

    await tester.tap(find.text('Analyze tracks…'));
    await tester.pumpAndSettle();
    expect(find.text('Analyze tracks…'), findsNothing);
  });

  testWidgets('MixarMenuAnchor mobile rows are flat, divided by hairlines', (
    tester,
  ) async {
    final theme = MixarThemeData.dark();
    await pumpMenuAnchor(tester, dialogTitle: 'Collection actions');
    await tester.tap(find.text('Open menu'));
    await tester.pumpAndSettle();

    final item = find.ancestor(
      of: find.text('Analyze tracks…'),
      matching: find.byType(MixarMenuItem),
    );
    expect(item, findsOneWidget);

    // No frame under the row: the list uses hairlines only.
    expect(
      find.descendant(
        of: item,
        matching: find.byWidgetPredicate((w) {
          if (w is! DecoratedBox) return false;
          final decoration = w.decoration;
          return decoration is BoxDecoration && decoration.border != null;
        }),
      ),
      findsNothing,
    );
    // Exactly one rule between the two rows, painted like the dialog's own
    // hairlines (not the darker MDivider default).
    expect(find.byType(MDivider), findsOneWidget);
    final divider = tester.widget<MDivider>(find.byType(MDivider));
    expect(divider.axis, Axis.horizontal);
    expect(divider.color, theme.colors.border);
  });

  testWidgets('desktop menu hover uses the shared quiet wash', (tester) async {
    final theme = MixarThemeData.dark();
    await pumpMenuAnchor(tester, style: SelectStyleSetting.desktop);
    await tester.tap(find.text('Open menu'));
    await tester.pumpAndSettle();

    final gesture = await tester.createGesture(kind: PointerDeviceKind.mouse);
    await gesture.addPointer(location: Offset.zero);
    addTearDown(gesture.removePointer);
    await tester.pump();

    await gesture.moveTo(tester.getCenter(find.text('Analyze tracks…')));
    await tester.pumpAndSettle();

    final wash = tester.widget<ColoredBox>(
      find
          .ancestor(
            of: find.text('Analyze tracks…'),
            matching: find.byType(ColoredBox),
          )
          .first,
    );
    expect(wash.color, theme.colors.secondary);
  });

  testWidgets('MixarMenuAnchor mobile barrier dismiss clears isShowing', (
    tester,
  ) async {
    final menuController = await pumpMenuAnchor(
      tester,
      captureController: true,
    );

    await tester.tap(find.text('Open menu'));
    await tester.pumpAndSettle();
    expect(find.text('Analyze tracks…'), findsOneWidget);
    expect(menuController!.isShowing, isTrue);

    // Barrier dismiss below the titlebar (same tap as mixar_dialog_test).
    await tester.tapAt(const Offset(8, kAppHeaderHeight + 10));
    await tester.pumpAndSettle();
    expect(find.text('Analyze tracks…'), findsNothing);
    expect(menuController.isShowing, isFalse);
  });

  testWidgets(
    'MixarMenuAnchor mobile hide then show waits for dialog to finish',
    (tester) async {
      final menuController = await pumpMenuAnchor(
        tester,
        captureController: true,
      );

      await tester.tap(find.text('Open menu'));
      await tester.pumpAndSettle();
      expect(find.text('Analyze tracks…'), findsOneWidget);

      // Rapid hide → show while the first dialog is still dismissing must not
      // stack a second route.
      menuController!.hide();
      menuController.show();
      await tester.pump();
      expect(find.text('Analyze tracks…'), findsOneWidget);

      await tester.pumpAndSettle();
      expect(find.text('Analyze tracks…'), findsNothing);
      expect(menuController.isShowing, isFalse);

      // After the route finishes, a fresh show works again.
      menuController.show();
      await tester.pumpAndSettle();
      expect(find.text('Analyze tracks…'), findsOneWidget);

      menuController.hide();
      await tester.pumpAndSettle();
      expect(find.text('Analyze tracks…'), findsNothing);
    },
  );
}
