import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/widgets/settings/settings_segmented_choice.dart';
import 'package:gui_flutter/widgets/shell/m_divider.dart';
import 'package:gui_flutter/widgets/shell/m_tappable.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:material_ui/material_ui.dart';

import 'support/mixar_material_app.dart';

void main() {
  Future<void> pumpChoice(
    WidgetTester tester,
    List<String> labels, {
    String value = 'One',
    ValueChanged<String>? onChanged,
    double width = 900,
  }) async {
    await tester.pumpWidget(
      mixarTestMaterialApp(
        theme: MixarThemeData.dark(),
        home: Scaffold(
          body: Center(
            child: SizedBox(
              width: width,
              child: SettingsSegmentedChoice<String>(
                value: value,
                options: [
                  for (final label in labels)
                    SettingsSegmentedOption<String>(value: label, label: label),
                ],
                onChanged: onChanged ?? (_) {},
              ),
            ),
          ),
        ),
      ),
    );
  }

  Finder horizontalDividers() => find.byWidgetPredicate(
    (widget) => widget is MDivider && widget.axis == Axis.horizontal,
  );
  Finder verticalDividers() => find.byWidgetPredicate(
    (widget) => widget is MDivider && widget.axis == Axis.vertical,
  );

  BorderRadiusGeometry? trackRadius(WidgetTester tester, Finder choice) {
    final track = tester.widget<DecoratedBox>(
      find.descendant(of: choice, matching: find.byType(DecoratedBox)).first,
    );
    return (track.decoration as BoxDecoration).borderRadius;
  }

  Finder cellFor(String label) =>
      find.ancestor(of: find.text(label), matching: find.byType(MTappable));

  /// The cell's foreground decoration (selection/focus border and shape).
  BoxDecoration decorationOf(WidgetTester tester, Finder cell) {
    final box = tester.widget<DecoratedBox>(
      find.descendant(
        of: cell,
        matching: find.byWidgetPredicate(
          (widget) =>
              widget is DecoratedBox &&
              widget.position == DecorationPosition.foreground,
        ),
      ),
    );
    return box.decoration as BoxDecoration;
  }

  BorderRadiusGeometry? borderRadiusOf(WidgetTester tester, Finder cell) =>
      decorationOf(tester, cell).borderRadius;

  Border? borderOf(WidgetTester tester, Finder cell) =>
      decorationOf(tester, cell).border as Border?;

  Color? fillOf(WidgetTester tester, Finder cell) {
    final box = tester.widget<DecoratedBox>(
      find.descendant(
        of: cell,
        matching: find.byWidgetPredicate(
          (widget) =>
              widget is DecoratedBox &&
              widget.position == DecorationPosition.background &&
              widget.decoration is BoxDecoration &&
              (widget.decoration as BoxDecoration).border == null,
        ),
      ),
    );
    return (box.decoration as BoxDecoration).color;
  }

  void expectRing(
    WidgetTester tester,
    Finder cell, {
    required double width,
    required Color color,
  }) {
    final border = borderOf(tester, cell);
    expect(border, isNotNull);
    expect(border!.top.width, width);
    expect(border.top.color, color);
  }

  testWidgets('up to five options stay in one rounded pill row', (
    tester,
  ) async {
    await pumpChoice(tester, const ['One', 'Two', 'Three', 'Four', 'Five']);

    final choice = find.byType(SettingsSegmentedChoice<String>);
    expect(
      trackRadius(tester, choice),
      MixarThemeData.dark().style.borderRadius.pill,
    );
    expect(horizontalDividers(), findsNothing);
    expect(verticalDividers(), findsNWidgets(4));

    final centres = {
      for (final label in ['One', 'Two', 'Three', 'Four', 'Five'])
        tester.getCenter(find.text(label)).dy.round(),
    };
    expect(centres, hasLength(1));
  }, semanticsEnabled: false);

  testWidgets('selection is a coloured cell border, not a fill', (
    tester,
  ) async {
    await pumpChoice(tester, const ['One', 'Two', 'Three']);

    expectRing(
      tester,
      cellFor('One'),
      width: 1.5,
      color: MixarThemeData.dark().colors.primary,
    );
    expect(borderOf(tester, cellFor('Two')), isNull);
    expect(fillOf(tester, cellFor('One')), const Color(0x00000000));

    // Only the edge cells round their outer corners, so the border follows
    // the pill instead of ending square.
    final pill = MixarThemeData.dark().style.borderRadius.pill;
    expect(
      borderRadiusOf(tester, cellFor('One')),
      BorderRadius.horizontal(left: pill.topLeft),
    );
    expect(borderRadiusOf(tester, cellFor('Two')), isNull);
    expect(
      borderRadiusOf(tester, cellFor('Three')),
      BorderRadius.horizontal(right: pill.topRight),
    );
  }, semanticsEnabled: false);

  testWidgets('a sixth option wraps the same cells into a three-column grid', (
    tester,
  ) async {
    await pumpChoice(tester, const [
      'One',
      'Two',
      'Three',
      'Four',
      'Five',
      'Six',
    ]);

    final choice = find.byType(SettingsSegmentedChoice<String>);
    // A multi-row track takes the softer card corner, not the stadium edge:
    // the pill radius would clip the corner cells.
    expect(
      trackRadius(tester, choice),
      MixarThemeData.dark().style.borderRadius.dialog,
    );
    final dialog = MixarThemeData.dark().style.borderRadius.dialog;

    // Each outer corner of the grid rounds only its own cell.
    expect(
      borderRadiusOf(tester, cellFor('One')),
      BorderRadius.only(topLeft: dialog.topLeft),
    );
    expect(
      borderRadiusOf(tester, cellFor('Three')),
      BorderRadius.only(topRight: dialog.topRight),
    );
    expect(
      borderRadiusOf(tester, cellFor('Four')),
      BorderRadius.only(bottomLeft: dialog.bottomLeft),
    );
    expect(
      borderRadiusOf(tester, cellFor('Six')),
      BorderRadius.only(bottomRight: dialog.bottomRight),
    );
    expect(horizontalDividers(), findsOneWidget);
    expect(verticalDividers(), findsNWidgets(4));

    final rowOne = {
      for (final label in ['One', 'Two', 'Three'])
        tester.getCenter(find.text(label)).dy.round(),
    };
    final rowTwo = {
      for (final label in ['Four', 'Five', 'Six'])
        tester.getCenter(find.text(label)).dy.round(),
    };
    expect(rowOne, hasLength(1));
    expect(rowTwo, hasLength(1));
    expect(rowOne.single, isNot(rowTwo.single));
    // A grid in a loose, bounded context stays compact instead of stretching
    // to the parent height.
    expect(
      tester.getSize(find.byType(SettingsSegmentedChoice<String>)).height,
      lessThan(160),
    );
  }, semanticsEnabled: false);

  testWidgets('a seven-option grid keeps the short last row inside the track', (
    tester,
  ) async {
    await pumpChoice(tester, const [
      'One',
      'Two',
      'Three',
      'Four',
      'Five',
      'Six',
      'Seven',
    ]);

    expect(tester.takeException(), isNull);
    expect(horizontalDividers(), findsNWidgets(2));
    // Two dividers per full row, none in the single-cell last row.
    expect(verticalDividers(), findsNWidgets(4));
    expect(find.text('Seven'), findsOneWidget);

    // The last real cell spans the leftover columns, owns the track's
    // bottom-right corner, and is not an inert placeholder.
    final seven = cellFor('Seven');
    expect(
      tester.getSize(seven).width,
      greaterThan(
        tester.getSize(find.byType(SettingsSegmentedChoice<String>)).width - 20,
      ),
    );
    final dialog = MixarThemeData.dark().style.borderRadius.dialog;
    expect(
      borderRadiusOf(tester, seven),
      BorderRadius.only(
        bottomLeft: dialog.topLeft,
        bottomRight: dialog.topRight,
      ),
    );
  }, semanticsEnabled: false);

  testWidgets('an eight-option grid gives the last pair the spare column', (
    tester,
  ) async {
    await pumpChoice(tester, const [
      'One',
      'Two',
      'Three',
      'Four',
      'Five',
      'Six',
      'Seven',
      'Eight',
    ]);

    expect(horizontalDividers(), findsNWidgets(2));
    // Rows of 3 + 3 + 2: two dividers, two dividers, one divider.
    expect(verticalDividers(), findsNWidgets(5));

    // The final cell takes the leftover two columns, not just one.
    final seven = tester.getSize(cellFor('Seven')).width;
    final eight = tester.getSize(cellFor('Eight')).width;
    expect(eight, closeTo(seven * 2, 2));
  }, semanticsEnabled: false);

  testWidgets('a very narrow pane stacks the options in one column', (
    tester,
  ) async {
    await pumpChoice(tester, const ['One', 'Two', 'Three'], width: 200);

    final choice = find.byType(SettingsSegmentedChoice<String>);
    expect(
      trackRadius(tester, choice),
      MixarThemeData.dark().style.borderRadius.dialog,
    );
    expect(verticalDividers(), findsNothing);
    expect(horizontalDividers(), findsNWidgets(2));
    expect(tester.takeException(), isNull);
  }, semanticsEnabled: false);

  testWidgets('tapping a grid cell emits its value', (tester) async {
    String? changed;
    await pumpChoice(tester, const [
      'One',
      'Two',
      'Three',
      'Four',
      'Five',
      'Six',
    ], onChanged: (value) => changed = value);

    await tester.tap(find.text('Five'));
    await tester.pump();

    expect(changed, 'Five');
  }, semanticsEnabled: false);

  testWidgets('a grid cell keeps its selected state in semantics', (
    tester,
  ) async {
    final handle = tester.ensureSemantics();
    await pumpChoice(tester, const [
      'One',
      'Two',
      'Three',
      'Four',
      'Five',
      'Six',
    ], value: 'Five');

    expect(
      tester.getSemantics(find.bySemanticsLabel('Five')),
      isSemantics(label: 'Five', isSelected: true, isButton: true),
    );
    handle.dispose();
  });

  testWidgets('a narrow pane wraps four options into a two-column grid', (
    tester,
  ) async {
    await pumpChoice(tester, const ['One', 'Two', 'Three', 'Four'], width: 320);

    final choice = find.byType(SettingsSegmentedChoice<String>);
    expect(
      trackRadius(tester, choice),
      MixarThemeData.dark().style.borderRadius.dialog,
    );
    expect(horizontalDividers(), findsOneWidget);
    expect(verticalDividers(), findsNWidgets(2));
    expect(tester.takeException(), isNull);
  }, semanticsEnabled: false);

  testWidgets('focus thickens the cell border', (tester) async {
    await pumpChoice(tester, const ['One', 'Two', 'Three']);

    await tester.sendKeyEvent(LogicalKeyboardKey.tab);
    await tester.pump();

    expectRing(
      tester,
      cellFor('One'),
      width: 2.5,
      color: MixarThemeData.dark().colors.primary,
    );

    // The next tab lands on an unselected cell, which gains the same ring.
    await tester.sendKeyEvent(LogicalKeyboardKey.tab);
    await tester.pump();

    expectRing(
      tester,
      cellFor('Two'),
      width: 2.5,
      color: MixarThemeData.dark().colors.foreground,
    );
  }, semanticsEnabled: false);
}
