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
  }) async {
    await tester.pumpWidget(
      mixarTestMaterialApp(
        theme: MixarThemeData.dark(),
        home: Scaffold(
          body: Center(
            child: SizedBox(
              width: 900,
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

    Finder cellFor(String label) =>
        find.ancestor(of: find.text(label), matching: find.byType(MTappable));
    Border? borderOf(Finder cell) {
      final box = tester.widget<DecoratedBox>(
        find.descendant(of: cell, matching: find.byType(DecoratedBox)).first,
      );
      return (box.decoration as BoxDecoration).border as Border?;
    }

    Color? fillOf(Finder cell) {
      final box = tester.widget<DecoratedBox>(
        find.descendant(of: cell, matching: find.byType(DecoratedBox)).at(1),
      );
      return (box.decoration as BoxDecoration).color;
    }

    final selected = borderOf(cellFor('One'));
    expect(selected, isNotNull);
    expect(selected!.top.width, 1.5);
    expect(selected.top.color, MixarThemeData.dark().colors.primary);
    expect(borderOf(cellFor('Two')), isNull);
    expect(fillOf(cellFor('One')), const Color(0x00000000));
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
    expect(verticalDividers(), findsNWidgets(6));
    expect(find.text('Seven'), findsOneWidget);
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
}
