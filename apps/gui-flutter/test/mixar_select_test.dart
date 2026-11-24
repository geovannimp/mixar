import 'package:flutter/gestures.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/theme/material_theme.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';
import 'package:gui_flutter/widgets/shell/app_header.dart';
import 'package:gui_flutter/widgets/shell/m_divider.dart';
import 'package:gui_flutter/widgets/shell/m_tappable.dart';
import 'package:gui_flutter/widgets/shell/mixar_menu_item.dart';
import 'package:gui_flutter/widgets/shell/mixar_select.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
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
    bool bottomAligned = false,
    bool topAligned = false,
  }) async {
    final theme = MixarThemeData.dark();
    await tester.pumpWidget(
      MaterialApp(
        theme: materialUiThemeFromMixar(theme),
        builder: mixarMaterialAppBuilder(theme),
        home: Scaffold(
          body: Align(
            alignment: topAligned
                ? Alignment.topCenter
                : bottomAligned
                ? Alignment.bottomCenter
                : Alignment.center,
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

  /// Finds the open menu panel by the borderless edge at its join.
  Finder popoverDecoratorFinder({required bool borderlessTop}) =>
      find.byWidgetPredicate(
        (widget) =>
            widget is ShadDecorator &&
            (borderlessTop
                ? widget.decoration?.border?.top == ShadBorderSide.none
                : widget.decoration?.border?.bottom == ShadBorderSide.none),
      );

  /// The select trigger's decorator, whose border and fill carry the chrome.
  ShadDecorator triggerDecorator(WidgetTester tester) =>
      tester.widget<ShadDecorator>(
        find
            .descendant(
              of: find.byType(ShadSelect<String>),
              matching: find.byType(ShadDecorator),
            )
            .first,
      );

  /// The background painted by one option row.
  Color optionFill(WidgetTester tester, String label) {
    // The trigger also shows the selected label; the option row is the later
    // occurrence in the open menu.
    final box = tester.widget<DecoratedBox>(
      find
          .ancestor(
            of: find.text(label).last,
            matching: find.byType(DecoratedBox),
          )
          .first,
    );
    final decoration = box.decoration;
    expect(decoration, isA<BoxDecoration>());
    return (decoration as BoxDecoration).color!;
  }

  testWidgets('the selected option is accent ink only, not a filled block', (
    tester,
  ) async {
    final theme = MixarThemeData.dark();
    await pumpSelect(tester, value: 'low', onChanged: (_) {});
    await openMenu(tester);

    // The focused selected row keeps the card; only the label and check take
    // the accent.
    expect(optionFill(tester, 'low'), const Color(0x00000000));
    final text = tester.widget<Text>(find.text('low').last);
    final style =
        text.style ??
        DefaultTextStyle.of(tester.element(find.text('low').last)).style;
    expect(style.color, theme.colors.primary);
    // The trigger's selected value stays in normal ink; accent is list-only.
    final triggerLabel = find.text('low').first;
    final triggerStyle = DefaultTextStyle.of(tester.element(triggerLabel))
        .style;
    expect(triggerStyle.color, theme.colors.foreground);
    final check = tester.widget<Icon>(
      find.descendant(
        of: find.byType(ShadOption<String>).first,
        matching: find.byIcon(LucideIcons.check),
      ),
    );
    expect(check.color, theme.colors.primary);
    expect(check.size, 14);
  });

  testWidgets('desktop options are menu bands with hairline dividers', (
    tester,
  ) async {
    final theme = MixarThemeData.dark();
    await pumpSelect(tester, value: 'low', onChanged: (_) {});
    await openMenu(tester);

    // One rule between each pair of choices, full-bleed like MixarMenuGroup.
    expect(find.byType(MDivider), findsNWidgets(2));
    for (final divider in tester.widgetList<MDivider>(find.byType(MDivider))) {
      expect(divider.color, theme.colors.border);
      expect(divider.padding, EdgeInsets.zero);
    }

    final row = tester.getRect(
      find
          .ancestor(
            of: find.text('high').last,
            matching: find.byType(ShadOption<String>),
          )
          .first,
    );
    expect(row.height, kMixarMenuRowHeight);

    final label = find.text('high').last;
    final style = DefaultTextStyle.of(tester.element(label)).style;
    expect(style.fontSize, theme.typography.body.sm.fontSize);
    expect(style.color, theme.colors.foreground);
  });

  testWidgets('hovering an option paints a dark-grey wash', (tester) async {
    final theme = MixarThemeData.dark();
    await pumpSelect(tester, value: 'low', onChanged: (_) {});
    await openMenu(tester);

    final gesture = await tester.createGesture(kind: PointerDeviceKind.mouse);
    await gesture.addPointer(location: Offset.zero);
    addTearDown(gesture.removePointer);
    await tester.pump();

    // Baseline: 'high' is neither selected nor hovered, so it must not carry
    // the wash before the pointer lands on it.
    expect(
      optionFill(tester, 'high'),
      const Color(0x00000000),
      reason: 'a non-selected, non-hovered option must stay untinted',
    );

    await gesture.moveTo(tester.getCenter(find.text('high')));
    await tester.pumpAndSettle();

    expect(optionFill(tester, 'high'), theme.colors.secondary);
  });

  testWidgets('keyboard focus paints a dark-grey wash on an unselected row', (
    tester,
  ) async {
    final theme = MixarThemeData.dark();
    await pumpSelect(tester, value: 'low', onChanged: (_) {});
    await openMenu(tester);

    final highLabel = find.text('high').last;
    Focus.of(tester.element(highLabel)).requestFocus();
    await tester.pumpAndSettle();

    expect(optionFill(tester, 'high'), theme.colors.secondary);
    // The selected row stays ink-only even while it owns focus on open.
    expect(optionFill(tester, 'low'), const Color(0x00000000));
  });

  testWidgets('desktop style still uses ShadSelect', (tester) async {
    await pumpSelect(tester, value: 'low', onChanged: (_) {});
    expect(find.byType(ShadSelect<String>), findsOneWidget);
  });

  testWidgets('closed select wears the full pill border', (tester) async {
    await pumpSelect(tester, value: 'low', onChanged: (_) {});

    final decorator = triggerDecorator(tester);
    final pill = MixarThemeData.dark().style.borderRadius.pill;
    expect(decorator.decoration?.border?.radius, pill);
    expect(decorator.decoration?.secondaryFocusedBorder?.radius, pill);
  });

  testWidgets('open menu joins the trigger into one card', (tester) async {
    await pumpSelect(tester, value: 'low', onChanged: (_) {});
    await openMenu(tester);

    final theme = MixarThemeData.dark();
    final trigger = triggerDecorator(tester);
    // The panel is the decorator whose join edge is borderless.
    final popoverFinder = popoverDecoratorFinder(borderlessTop: true);
    final popover = tester.widget<ShadDecorator>(popoverFinder);
    final triggerRect = tester.getRect(find.byType(ShadSelect<String>));
    final popoverRect = tester.getRect(popoverFinder);

    // The trigger squares off at the join and keeps the closed pill's top half
    // as the card's cap.
    expect(
      trigger.decoration?.border?.radius,
      BorderRadius.vertical(top: Radius.circular(triggerRect.height / 2)),
    );
    expect(trigger.decoration?.color, theme.colors.card);
    // The panel rounds only its bottom corners; the trigger's bottom stroke
    // draws the divider.
    expect(
      popover.decoration?.border?.radius,
      BorderRadius.vertical(bottom: theme.style.borderRadius.dialog.topLeft),
    );
    expect(popover.decoration?.border?.top, ShadBorderSide.none);

    // The panel starts exactly at the trigger's bottom edge and its side
    // strokes line up with the trigger's.
    expect(popoverRect.top, moreOrLessEquals(triggerRect.bottom, epsilon: 0.5));
    expect(popoverRect.left, moreOrLessEquals(triggerRect.left, epsilon: 0.5));
    expect(
      popoverRect.width,
      moreOrLessEquals(triggerRect.width, epsilon: 0.5),
    );
  });

  testWidgets('menu near the window bottom joins above the trigger', (
    tester,
  ) async {
    tester.view.physicalSize = const Size(800, 300);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);

    await pumpSelect(
      tester,
      value: 'low',
      onChanged: (_) {},
      bottomAligned: true,
    );
    final labelBefore = tester.getRect(find.text('low').first);
    await openMenu(tester);

    final theme = MixarThemeData.dark();
    final trigger = triggerDecorator(tester);
    final popoverFinder = popoverDecoratorFinder(borderlessTop: false);
    final popover = tester.widget<ShadDecorator>(popoverFinder);
    final triggerRect = tester.getRect(find.byType(ShadSelect<String>));
    final popoverRect = tester.getRect(popoverFinder);

    // The trigger keeps its bottom cap; the menu rounds its top corners.
    expect(
      trigger.decoration?.border?.radius,
      BorderRadius.vertical(bottom: Radius.circular(triggerRect.height / 2)),
    );
    expect(
      popover.decoration?.border?.radius,
      BorderRadius.vertical(top: theme.style.borderRadius.dialog.topLeft),
    );
    expect(popover.decoration?.border?.bottom, ShadBorderSide.none);
    expect(popover.decoration?.border?.top, isNot(ShadBorderSide.none));

    // Flush above, same width, and the trigger content did not move.
    expect(popoverRect.bottom, moreOrLessEquals(triggerRect.top, epsilon: 0.5));
    expect(popoverRect.left, moreOrLessEquals(triggerRect.left, epsilon: 0.5));
    expect(
      popoverRect.width,
      moreOrLessEquals(triggerRect.width, epsilon: 0.5),
    );
    expect(tester.getRect(find.text('low').first), labelBefore);
  });

  testWidgets('opening the menu does not move the trigger label or chevron', (
    tester,
  ) async {
    await pumpSelect(tester, value: 'low', onChanged: (_) {});

    final labelBefore = tester.getRect(find.text('low').first);
    final chevron = find.byIcon(LucideIcons.chevronDown).first;
    final chevronBefore = tester.getRect(chevron);

    await openMenu(tester);

    expect(tester.getRect(find.text('low').first), labelBefore);
    expect(tester.getRect(chevron), chevronBefore);
  });

  testWidgets('a scrolled long list keeps the join below the trigger', (
    tester,
  ) async {
    final options = [for (var i = 0; i < 40; i++) 'option $i'];
    await pumpSelect(
      tester,
      value: options.last,
      options: options,
      topAligned: true,
      onChanged: (_) {},
    );
    await openMenu(tester);

    final trigger = triggerDecorator(tester);
    final triggerRect = tester.getRect(find.byType(ShadSelect<String>));
    final popoverFinder = popoverDecoratorFinder(borderlessTop: true);
    final popoverRect = tester.getRect(popoverFinder);

    // The selected last row scrolled the first row above the trigger; the
    // panel's own top still sits below it, so the trigger keeps its top cap.
    expect(
      tester.getRect(find.text('option 0')).top,
      lessThan(triggerRect.top),
    );
    expect(
      trigger.decoration?.border?.radius,
      BorderRadius.vertical(top: Radius.circular(triggerRect.height / 2)),
    );
    expect(popoverRect.top, moreOrLessEquals(triggerRect.bottom, epsilon: 0.5));
  });

  testWidgets('closing the menu restores the trigger pill', (tester) async {
    await pumpSelect(tester, value: 'low', onChanged: (_) {});
    await openMenu(tester);
    await tester.tapAt(const Offset(5, 5));
    await tester.pumpAndSettle();

    final trigger = triggerDecorator(tester);
    expect(
      trigger.decoration?.border?.radius,
      MixarThemeData.dark().style.borderRadius.pill,
    );
    expect(trigger.decoration?.color, isNull);
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

  testWidgets('mobile style lists options with dividers instead of cards', (
    tester,
  ) async {
    await pumpSelect(
      tester,
      value: 'low',
      onChanged: (_) {},
      style: SelectStyleSetting.mobile,
      dialogTitle: 'Quality',
    );
    await openMenu(tester);

    // One hairline between each pair of options, none above the first row.
    expect(find.byType(MDivider), findsNWidgets(2));

    // Rows are flat: no frame stroke or corner radius around the label.
    final row = tester.widget<DecoratedBox>(
      find
          .ancestor(of: find.text('high'), matching: find.byType(DecoratedBox))
          .first,
    );
    final decoration = row.decoration as BoxDecoration;
    expect(decoration.border, isNull);
    expect(decoration.borderRadius, isNull);
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

    // Barrier dismiss below the titlebar (same tap as mixar_dialog_test).
    await tester.tapAt(const Offset(8, kAppHeaderHeight + 10));
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

    final open = tester
        .widget<ShadSelect<String>>(find.byType(ShadSelect<String>))
        .onPressed;
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
    expect(subtitle.style?.color, theme.colors.mutedForeground);
    expect(subtitle.textAlign, TextAlign.start);

    final title = tester.widget<Text>(find.text('Mobile').last);
    expect(title.textAlign, TextAlign.start);
    expect(title.style?.color, theme.colors.primary);
    expect(title.style?.fontWeight, FontWeight.w600);

    final selectedRow = tester.widget<DecoratedBox>(
      find
          .ancestor(
            of: find.text('Dialog picker'),
            matching: find.byType(DecoratedBox),
          )
          .first,
    );
    final decoration = selectedRow.decoration as BoxDecoration;
    expect(decoration.color, theme.colors.card);
    expect(
      find.descendant(
        of: find.ancestor(
          of: find.text('Dialog picker'),
          matching: find.byType(MTappable),
        ),
        matching: find.byIcon(LucideIcons.check),
      ),
      findsOneWidget,
    );
  });
}
