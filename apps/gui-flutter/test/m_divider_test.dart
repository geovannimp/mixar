import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/shell/m_divider.dart';
import 'package:gui_flutter/shell/material_theme.dart';
import 'package:material_ui/material_ui.dart';
import 'package:gui_flutter/shell/mixar_theme.dart';

import 'support/mixar_material_app.dart';

void main() {
  testWidgets(
    'MDivider hairline uses secondary + borderWidth + default padding',
    (tester) async {
      final base = MixarThemeData.dark();
      final theme = base;
      await tester.pumpWidget(
        MaterialApp(
          theme: materialUiThemeFromMixar(theme),
          builder: mixarMaterialAppBuilder(base),
          home: const Scaffold(
            body: SizedBox(width: 200, child: Column(children: [MDivider()])),
          ),
        ),
      );

      final container = tester.widget<Container>(
        find.descendant(
          of: find.byType(MDivider),
          matching: find.byType(Container),
        ),
      );
      expect(container.color, theme.colors.secondary);
      expect(container.margin, const EdgeInsets.symmetric(vertical: 16));
      expect(
        tester.getSize(find.byType(MDivider)).height,
        theme.style.borderWidth + 32,
      );
    },
  );

  testWidgets('MDivider respects zero padding', (tester) async {
    final base = MixarThemeData.dark();
    final theme = base;
    await tester.pumpWidget(
      MaterialApp(
        theme: materialUiThemeFromMixar(theme),
        builder: mixarMaterialAppBuilder(base),
        home: const Scaffold(
          body: SizedBox(
            width: 200,
            child: Column(children: [MDivider(padding: EdgeInsets.zero)]),
          ),
        ),
      ),
    );

    expect(
      tester.getSize(find.byType(MDivider)).height,
      theme.style.borderWidth,
    );
  });
}
