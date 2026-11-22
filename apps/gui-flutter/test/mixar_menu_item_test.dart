import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/theme/material_theme.dart';
import 'package:gui_flutter/widgets/shell/mixar_menu_item.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:material_ui/material_ui.dart';

import 'support/mixar_material_app.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  Future<void> pumpItem(WidgetTester tester, {required bool selected}) async {
    final theme = MixarThemeData.dark();
    await tester.pumpWidget(
      MaterialApp(
        theme: materialUiThemeFromMixar(theme),
        builder: mixarMaterialAppBuilder(theme),
        home: Scaffold(
          // Loose height, like a menu column: the row is a band, not a fill.
          body: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              MixarMenuItem(
                title: const Text('Echo'),
                selected: selected,
                onPress: () {},
              ),
            ],
          ),
        ),
      ),
    );
  }

  testWidgets('a text row keeps the menu band height', (tester) async {
    await pumpItem(tester, selected: false);

    expect(
      tester.getSize(find.byType(MixarMenuItem)).height,
      kMixarMenuRowHeight,
    );
  });

  testWidgets('a selected row announces its state', (tester) async {
    final semantics = tester.ensureSemantics();
    await pumpItem(tester, selected: true);

    expect(
      tester.getSemantics(find.byType(MixarMenuItem)),
      isSemantics(label: 'Echo', isButton: true, isSelected: true),
    );
    semantics.dispose();
  });

  testWidgets('a plain row does not announce selected', (tester) async {
    final semantics = tester.ensureSemantics();
    await pumpItem(tester, selected: false);

    expect(
      tester.getSemantics(find.byType(MixarMenuItem)),
      isNot(isSemantics(isSelected: true)),
    );
    semantics.dispose();
  });
}
