import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/theme/material_theme.dart';
import 'package:gui_flutter/widgets/shell/m_divider.dart';
import 'package:gui_flutter/widgets/shell/mixar_menu_group.dart';
import 'package:gui_flutter/widgets/shell/mixar_menu_item.dart';
import 'package:gui_flutter/widgets/shell/mixar_menu_label.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:material_ui/material_ui.dart';

import 'support/mixar_material_app.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  Future<void> pumpGroup(WidgetTester tester, List<Widget> children) async {
    final theme = MixarThemeData.dark();
    await tester.pumpWidget(
      MaterialApp(
        theme: materialUiThemeFromMixar(theme),
        builder: mixarMaterialAppBuilder(theme),
        home: Scaffold(body: MixarMenuGroup(children: children)),
      ),
    );
  }

  MixarMenuItem row(String label) =>
      MixarMenuItem(title: Text(label), onPress: () {});

  testWidgets('adjacent rows get one hairline', (tester) async {
    await pumpGroup(tester, [row('One'), row('Two')]);

    expect(find.byType(MDivider), findsOneWidget);
    expect(
      tester.widget<MDivider>(find.byType(MDivider)).axis,
      Axis.horizontal,
    );
  });

  testWidgets('a section label is not separated from its rows', (tester) async {
    await pumpGroup(tester, [
      const MixarMenuLabel(child: Text('Section')),
      row('One'),
    ]);

    expect(find.byType(MDivider), findsNothing);
  });

  testWidgets('a nested group owns its own rules', (tester) async {
    await pumpGroup(tester, [
      row('One'),
      MixarMenuGroup(children: [row('Two'), row('Three')]),
      row('Four'),
    ]);

    // Only the inner pair is divided; the parent adds no rules around the
    // nested block.
    expect(find.byType(MDivider), findsOneWidget);
  });
}
