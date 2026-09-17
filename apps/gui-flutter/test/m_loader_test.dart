import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/shell/m_loader.dart';
import 'package:gui_flutter/shell/material_theme.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';
import 'package:material_ui/material_ui.dart';
import 'package:gui_flutter/shell/mixar_theme.dart';

import 'support/mixar_material_app.dart';

void main() {
  testWidgets(
    'MLoader uses loaderCircle at md body size; color override wins',
    (tester) async {
      final base = MixarThemeData.dark();
      final theme = base;
      const override = Color(0xFF00FF00);
      await tester.pumpWidget(
        MaterialApp(
          theme: materialUiThemeFromMixar(theme),
          builder: mixarMaterialAppBuilder(base),
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
