import 'package:flutter_test/flutter_test.dart';
import 'package:forui/forui.dart';
import 'package:gui_flutter/shell/app_typography.dart';
import 'package:gui_flutter/shell/m_divider.dart';
import 'package:gui_flutter/shell/material_theme.dart';
import 'package:material_ui/material_ui.dart';

import 'support/forui_material_app.dart';

void main() {
  testWidgets(
    'MDivider hairline uses secondary + borderWidth + default padding',
    (tester) async {
      final base = FTheme.neutral.dark.desktop;
      final theme = mixarThemeData(base, touch: false);
      await tester.pumpWidget(
        MaterialApp(
          theme: materialUiThemeFromForui(theme),
          builder: foruiMaterialAppBuilder(base),
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
    final base = FTheme.neutral.dark.desktop;
    final theme = mixarThemeData(base, touch: false);
    await tester.pumpWidget(
      MaterialApp(
        theme: materialUiThemeFromForui(theme),
        builder: foruiMaterialAppBuilder(base),
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
