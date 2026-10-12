import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/shell/m_card.dart';
import 'package:gui_flutter/shell/material_theme.dart';
import 'package:gui_flutter/shell/mixar_theme.dart';
import 'package:material_ui/material_ui.dart';

import 'support/mixar_material_app.dart';

void main() {
  testWidgets('MCard paints card fill + square border', (tester) async {
    final base = MixarThemeData.dark();
    final theme = base;
    await tester.pumpWidget(
      MaterialApp(
        theme: materialUiThemeFromMixar(theme),
        builder: mixarMaterialAppBuilder(base),
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
    final base = MixarThemeData.dark();
    await tester.pumpWidget(
      MaterialApp(
        theme: materialUiThemeFromMixar(base),
        builder: mixarMaterialAppBuilder(base),
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

  testWidgets('MCard accepts a border color override', (tester) async {
    final base = MixarThemeData.dark();
    const override = Color(0xFF34D399);
    await tester.pumpWidget(
      MaterialApp(
        theme: materialUiThemeFromMixar(base),
        builder: mixarMaterialAppBuilder(base),
        home: const Scaffold(
          body: MCard(
            borderColor: override,
            child: SizedBox(width: 40, height: 40),
          ),
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
    expect(
      decoration.border,
      Border.all(color: override, width: base.style.borderWidth),
    );
  });

  testWidgets('MCard insets children inside the hairline border', (
    tester,
  ) async {
    final base = MixarThemeData.dark();
    const childKey = ValueKey('m-card-child');
    await tester.pumpWidget(
      MaterialApp(
        theme: materialUiThemeFromMixar(base),
        builder: mixarMaterialAppBuilder(base),
        home: const Scaffold(
          body: Align(
            alignment: Alignment.topLeft,
            child: MCard(child: SizedBox(key: childKey, width: 40, height: 40)),
          ),
        ),
      ),
    );

    final cardRect = tester.getRect(find.byType(MCard));
    final childRect = tester.getRect(find.byKey(childKey));
    final inset = base.style.borderWidth;
    expect(childRect.left, cardRect.left + inset);
    expect(childRect.top, cardRect.top + inset);
    expect(childRect.right, cardRect.right - inset);
    expect(childRect.bottom, cardRect.bottom - inset);
  });
}
