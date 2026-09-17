import 'package:flutter_test/flutter_test.dart';
import 'package:forui/forui.dart';
import 'package:gui_flutter/shell/material_theme.dart';
import 'package:gui_flutter/shell/mixar_button_style.dart';
import 'package:gui_flutter/shell/mixar_dialog.dart';
import 'package:material_ui/material_ui.dart';

import 'support/forui_material_app.dart';

void main() {
  Future<void> pumpHome(WidgetTester tester, Widget home) async {
    final theme = FTheme.neutral.dark.desktop;
    await tester.pumpWidget(
      MaterialApp(
        theme: materialUiThemeFromForui(theme),
        builder: foruiMaterialAppBuilder(theme),
        home: Scaffold(body: home),
      ),
    );
  }

  testWidgets('showMixarConfirm returns chosen action value', (tester) async {
    String? result;
    await pumpHome(
      tester,
      Builder(
        builder: (context) => GestureDetector(
          onTap: () async {
            result = await showMixarConfirm<String>(
              context: context,
              title: 'Delete?',
              body: 'Cannot undo.',
              actions: const [
                MixarDialogAction(
                  label: 'Cancel',
                  value: 'cancel',
                  variant: MixarButtonVariant.outline,
                ),
                MixarDialogAction(
                  label: 'Delete',
                  value: 'delete',
                  variant: MixarButtonVariant.destructive,
                ),
              ],
            );
          },
          child: const Text('Open'),
        ),
      ),
    );

    await tester.tap(find.text('Open'));
    await tester.pumpAndSettle();
    expect(find.text('Delete?'), findsWidgets);
    await tester.tap(find.text('Delete'));
    await tester.pumpAndSettle();
    expect(result, 'delete');
  });

  testWidgets('showMixarConfirm barrier dismiss returns null', (tester) async {
    String? result = 'unset';
    await pumpHome(
      tester,
      Builder(
        builder: (context) => GestureDetector(
          onTap: () async {
            result = await showMixarConfirm<String>(
              context: context,
              title: 'Hello',
              actions: const [MixarDialogAction(label: 'Ok', value: 'ok')],
            );
          },
          child: const Text('Open'),
        ),
      ),
    );

    await tester.tap(find.text('Open'));
    await tester.pumpAndSettle();
    await tester.tapAt(const Offset(8, 8));
    await tester.pumpAndSettle();
    expect(result, isNull);
  });
}
