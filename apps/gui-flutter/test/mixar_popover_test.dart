import 'package:flutter_test/flutter_test.dart';
import 'package:forui/forui.dart';
import 'package:gui_flutter/shell/material_theme.dart';
import 'package:gui_flutter/shell/mixar_menu.dart';
import 'package:gui_flutter/shell/mixar_popover.dart';
import 'package:material_ui/material_ui.dart';

import 'support/forui_material_app.dart';

void main() {
  testWidgets('MixarPopover toggles overlay content', (tester) async {
    final theme = FTheme.neutral.dark.desktop;
    await tester.pumpWidget(
      MaterialApp(
        theme: materialUiThemeFromForui(theme),
        builder: foruiMaterialAppBuilder(theme),
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

    await tester.tap(find.text('Open'));
    await tester.pumpAndSettle();
    expect(find.text('Gain panel'), findsNothing);
  });

  testWidgets('MixarMenuPanel keeps a fixed width', (tester) async {
    final theme = FTheme.neutral.dark.desktop;
    await tester.pumpWidget(
      MaterialApp(
        theme: materialUiThemeFromForui(theme),
        builder: foruiMaterialAppBuilder(theme),
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
}
