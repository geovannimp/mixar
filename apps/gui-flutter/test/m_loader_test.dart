import 'package:flutter_test/flutter_test.dart';
import 'package:forui/forui.dart';
import 'package:gui_flutter/shell/app_typography.dart';
import 'package:gui_flutter/shell/m_loader.dart';
import 'package:gui_flutter/shell/material_theme.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';
import 'package:material_ui/material_ui.dart';

import 'support/forui_material_app.dart';

void main() {
  testWidgets(
    'MLoader uses loaderCircle at md body size; color override wins',
    (tester) async {
      final base = FTheme.neutral.dark.desktop;
      final theme = mixarThemeData(base, touch: false);
      const override = Color(0xFF00FF00);
      await tester.pumpWidget(
        MaterialApp(
          theme: materialUiThemeFromForui(theme),
          builder: foruiMaterialAppBuilder(base),
          home: const Scaffold(
            body: IconTheme(
              data: IconThemeData(color: Color(0xFFFF0000), size: 99),
              child: MLoader(color: override),
            ),
          ),
        ),
      );
      await tester.pump();

      final icon = tester.widget<Icon>(find.byIcon(LucideIcons.loaderCircle));
      expect(icon.color, override);
      expect(icon.size, theme.typography.body.md.fontSize);
    },
  );
}
