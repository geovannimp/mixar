import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/shell/material_theme.dart';
import 'package:gui_flutter/shell/mixar_menu.dart';
import 'package:gui_flutter/shell/mixar_popover.dart';
import 'package:gui_flutter/shell/mixar_theme.dart';
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
}
