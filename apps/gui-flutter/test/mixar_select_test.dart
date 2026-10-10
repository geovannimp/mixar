import 'package:flutter/gestures.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/shell/material_theme.dart';
import 'package:gui_flutter/shell/mixar_select.dart';
import 'package:gui_flutter/shell/mixar_theme.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';
import 'package:material_ui/material_ui.dart';
import 'package:shadcn_ui/shadcn_ui.dart';

import 'support/mixar_material_app.dart';

void main() {
  Future<void> pumpSelect(
    WidgetTester tester, {
    required String value,
    required void Function(String) onChanged,
    SelectStyleSetting style = SelectStyleSetting.desktop,
    String? dialogTitle,
    bool enabled = true,
    List<String> options = const ['low', 'medium', 'high'],
    String Function(String value)? labelBuilder,
    String Function(String value)? subtitleBuilder,
    double width = 220,
  }) async {
    final theme = MixarThemeData.dark();
    await tester.pumpWidget(
      MaterialApp(
        theme: materialUiThemeFromMixar(theme),
        builder: mixarMaterialAppBuilder(theme),
        home: Scaffold(
          body: Center(
            child: SizedBox(
              width: width,
              child: MixarSelect<String>(
                value: value,
                options: options,
                labelBuilder: labelBuilder ?? (option) => option,
                subtitleBuilder: subtitleBuilder,
                onChanged: onChanged,
                style: style,
                dialogTitle: dialogTitle,
                enabled: enabled,
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

  testWidgets('desktop style still uses ShadSelect', (tester) async {
    await pumpSelect(tester, value: 'low', onChanged: (_) {});
    expect(find.byType(ShadSelect<String>), findsOneWidget);
  });

  testWidgets('mobile style opens a dialog of options and selects', (
    tester,
  ) async {
    String? selected;
    await pumpSelect(
      tester,
      value: 'low',
      onChanged: (next) => selected = next,
      style: SelectStyleSetting.mobile,
      dialogTitle: 'Quality',
    );

    // Closed chrome stays ShadSelect; only open behavior changes.
    expect(find.byType(ShadSelect<String>), findsOneWidget);
    await tester.tap(find.byType(ShadSelect<String>));
    await tester.pumpAndSettle();

    expect(find.text('Quality'), findsOneWidget);
    expect(find.text('medium'), findsOneWidget);
    await tester.tap(find.text('medium'));
    await tester.pumpAndSettle();

    expect(selected, 'medium');
    expect(find.text('Quality'), findsNothing);
  });

  testWidgets('mobile style dismiss without selection skips onChanged', (
    tester,
  ) async {
    var changed = false;
    await pumpSelect(
      tester,
      value: 'low',
      onChanged: (_) => changed = true,
      style: SelectStyleSetting.mobile,
      dialogTitle: 'Quality',
    );

    await tester.tap(find.byType(ShadSelect<String>));
    await tester.pumpAndSettle();
    expect(find.text('Quality'), findsOneWidget);

    // Barrier dismiss (same corner tap as mixar_dialog_test).
    await tester.tapAt(const Offset(8, 8));
    await tester.pumpAndSettle();

    expect(find.text('Quality'), findsNothing);
    expect(changed, isFalse);
  });

  testWidgets('disabled mobile style does not open a dialog', (tester) async {
    await pumpSelect(
      tester,
      value: 'low',
      onChanged: (_) {},
      style: SelectStyleSetting.mobile,
      dialogTitle: 'Quality',
      enabled: false,
    );

    await tester.tap(find.byType(ShadSelect<String>));
    await tester.pumpAndSettle();
    expect(find.text('Quality'), findsNothing);
  });

  testWidgets('rapid mobile taps open only one dialog', (tester) async {
    var changes = 0;
    await pumpSelect(
      tester,
      value: 'low',
      onChanged: (_) => changes++,
      style: SelectStyleSetting.mobile,
      dialogTitle: 'Quality',
    );

    final open = tester.widget<ShadSelect<String>>(
      find.byType(ShadSelect<String>),
    ).onPressed;
    expect(open, isNotNull);
    // Two opens in the same turn: the second must hit `_isOpening` and no-op
    // (both run synchronously until the first `await showMixarDialog`).
    open!();
    open();
    await tester.pumpAndSettle();

    expect(find.text('Quality'), findsOneWidget);
    await tester.tap(find.text('medium'));
    await tester.pumpAndSettle();

    expect(changes, 1);
    expect(find.text('Quality'), findsNothing);
  });

  testWidgets('mobile option subtitles stay left-aligned and muted', (
    tester,
  ) async {
    final theme = MixarThemeData.dark();
    await pumpSelect(
      tester,
      value: 'mobile',
      onChanged: (_) {},
      style: SelectStyleSetting.mobile,
      options: const ['auto', 'desktop', 'mobile'],
      labelBuilder: (option) => switch (option) {
        'auto' => 'Auto',
        'desktop' => 'Desktop',
        _ => 'Mobile',
      },
      subtitleBuilder: (option) => switch (option) {
        'auto' => 'Platform default',
        'desktop' => 'Popover dropdown',
        _ => 'Dialog picker',
      },
      width: 280,
    );

    expect(find.byType(ShadSelect<String>), findsOneWidget);
    await tester.tap(find.byType(ShadSelect<String>));
    await tester.pumpAndSettle();

    final subtitle = tester.widget<Text>(find.text('Dialog picker'));
    expect(
      subtitle.style?.color,
      theme.colors.selectionForeground.withValues(alpha: 0.75),
    );
    expect(subtitle.textAlign, TextAlign.start);

    final title = tester.widget<Text>(find.text('Mobile').last);
    expect(title.textAlign, TextAlign.start);
    expect(title.style?.color, theme.colors.selectionForeground);

    final selectedRow = tester.widget<DecoratedBox>(
      find
          .ancestor(
            of: find.text('Dialog picker'),
            matching: find.byType(DecoratedBox),
          )
          .first,
    );
    final decoration = selectedRow.decoration! as BoxDecoration;
    expect(decoration.color, theme.colors.selection);
  });
}
