import 'package:flutter_test/flutter_test.dart';
import 'package:forui/forui.dart';
import 'package:gui_flutter/shell/app_typography.dart';
import 'package:gui_flutter/shell/m_card.dart';
import 'package:gui_flutter/shell/material_theme.dart';
import 'package:material_ui/material_ui.dart';

import 'support/forui_material_app.dart';

void main() {
  testWidgets('MCard paints card fill + rounded border', (tester) async {
    final base = FTheme.neutral.dark.desktop;
    final theme = mixarThemeData(base, touch: false);
    await tester.pumpWidget(
      MaterialApp(
        theme: materialUiThemeFromForui(theme),
        builder: foruiMaterialAppBuilder(base),
        home: const Scaffold(
          body: MCard(child: SizedBox(width: 40, height: 40)),
        ),
      ),
    );

    final decorated = tester.widget<DecoratedBox>(
      find.descendant(
        of: find.byType(MCard),
        matching: find.byType(DecoratedBox),
      ),
    );
    final decoration = decorated.decoration as BoxDecoration;
    expect(decoration.color, theme.colors.card);
    expect(decoration.borderRadius, theme.style.borderRadius.lg);
    expect(
      decoration.border,
      Border.all(color: theme.colors.border, width: theme.style.borderWidth),
    );
    expect(find.byType(ClipRRect), findsNothing);
  });

  testWidgets('MCard clips with ClipRRect when requested', (tester) async {
    final base = FTheme.neutral.dark.desktop;
    await tester.pumpWidget(
      MaterialApp(
        theme: materialUiThemeFromForui(mixarThemeData(base, touch: false)),
        builder: foruiMaterialAppBuilder(base),
        home: const Scaffold(
          body: MCard(
            clipBehavior: Clip.antiAlias,
            child: SizedBox(width: 40, height: 40),
          ),
        ),
      ),
    );

    expect(find.byType(ClipRRect), findsOneWidget);
  });
}
