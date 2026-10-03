import 'package:flutter/gestures.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/shell/material_theme.dart';
import 'package:gui_flutter/shell/mixar_select.dart';
import 'package:gui_flutter/shell/mixar_theme.dart';
import 'package:material_ui/material_ui.dart';
import 'package:shadcn_ui/shadcn_ui.dart';

import 'support/mixar_material_app.dart';

void main() {
  Future<void> pumpSelect(
    WidgetTester tester, {
    required String value,
    required void Function(String) onChanged,
  }) async {
    final theme = MixarThemeData.dark();
    await tester.pumpWidget(
      MaterialApp(
        theme: materialUiThemeFromMixar(theme),
        builder: mixarMaterialAppBuilder(theme),
        home: Scaffold(
          body: Center(
            child: SizedBox(
              width: 220,
              child: MixarSelect<String>(
                value: value,
                options: const ['low', 'medium', 'high'],
                labelBuilder: (option) => option,
                onChanged: onChanged,
              ),
            ),
          ),
        ),
      ),
    );
  }

  Future<void> openMenu(WidgetTester tester) async {
    await tester.tap(find.byType(ShadSelect<String>));
    await tester.pumpAndSettle();
  }

  /// The `Container` decoration painted behind one option row.
  Color optionFill(WidgetTester tester, String label) {
    final containers = tester
        .widgetList<Container>(
          find.ancestor(of: find.text(label), matching: find.byType(Container)),
        )
        .where(
          (c) =>
              c.decoration is BoxDecoration &&
              (c.decoration! as BoxDecoration).color != null,
        )
        .toList();
    expect(
      containers,
      isNotEmpty,
      reason: 'no decorated option row found for "$label"',
    );
    final decoration = containers.first.decoration! as BoxDecoration;
    return decoration.color!;
  }

  testWidgets('the selected option uses the Mixar selection tint, not a solid '
      'accent block', (tester) async {
    final theme = MixarThemeData.dark();
    await pumpSelect(tester, value: 'low', onChanged: (_) {});
    await openMenu(tester);

    expect(optionFill(tester, 'low'), theme.colors.selection);
    // The stock Shad default was `colorScheme.accent`, a solid teal block.
    expect(optionFill(tester, 'low'), isNot(theme.colors.accent));
  });

  testWidgets('the selected option text is legible over the tint', (
    tester,
  ) async {
    final theme = MixarThemeData.dark();
    await pumpSelect(tester, value: 'low', onChanged: (_) {});
    await openMenu(tester);

    final text = tester.widget<Text>(find.text('low').last);
    final style =
        text.style ??
        DefaultTextStyle.of(tester.element(find.text('low').last)).style;
    expect(style.color, theme.colors.selectionForeground);
  });

  testWidgets('hovering an option paints the same quiet selection tint', (
    tester,
  ) async {
    final theme = MixarThemeData.dark();
    await pumpSelect(tester, value: 'low', onChanged: (_) {});
    await openMenu(tester);

    final gesture = await tester.createGesture(kind: PointerDeviceKind.mouse);
    await gesture.addPointer(location: Offset.zero);
    addTearDown(gesture.removePointer);
    await tester.pump();

    // Baseline: 'high' is neither selected nor hovered, so it must not carry
    // the tint before the pointer lands on it.
    expect(
      optionFill(tester, 'high'),
      isNot(theme.colors.selection),
      reason: 'a non-selected, non-hovered option must stay untinted',
    );

    await gesture.moveTo(tester.getCenter(find.text('high')));
    await tester.pumpAndSettle();

    expect(optionFill(tester, 'high'), theme.colors.selection);
  });
}
