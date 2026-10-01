import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/shell/m_tabs.dart';
import 'package:gui_flutter/shell/material_theme.dart';
import 'package:gui_flutter/shell/mixar_theme.dart';
import 'package:material_ui/material_ui.dart';

import 'support/mixar_material_app.dart';

void main() {
  Future<void> pumpTabs(
    WidgetTester tester, {
    required List<MTabEntry> children,
    Axis direction = Axis.horizontal,
    int? index,
    ValueChanged<int>? onChange,
    BorderSide? barBottomBorder,
  }) async {
    final theme = MixarThemeData.dark();
    await tester.pumpWidget(
      MaterialApp(
        theme: materialUiThemeFromMixar(theme),
        builder: mixarMaterialAppBuilder(theme),
        home: Scaffold(
          body: SizedBox(
            width: 240,
            height: 200,
            child: MTabs(
              direction: direction,
              expands: true,
              index: index,
              onChange: onChange,
              barBottomBorder: barBottomBorder,
              children: children,
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();
  }

  testWidgets('uncontrolled tabs switch IndexedStack content', (tester) async {
    await pumpTabs(
      tester,
      children: const [
        MTabEntry(label: Text('A'), child: Text('pane-a')),
        MTabEntry(label: Text('B'), child: Text('pane-b')),
      ],
    );

    expect(find.text('pane-a'), findsOneWidget);
    expect(find.text('pane-b'), findsNothing);

    await tester.tap(find.text('B'));
    await tester.pumpAndSettle();

    expect(find.text('pane-b'), findsOneWidget);
    expect(find.text('pane-a'), findsNothing);
  });

  testWidgets('controlled index follows parent + onChange', (tester) async {
    var index = 0;
    late void Function(void Function()) setParent;

    final theme = MixarThemeData.dark();
    await tester.pumpWidget(
      MaterialApp(
        theme: materialUiThemeFromMixar(theme),
        builder: mixarMaterialAppBuilder(theme),
        home: Scaffold(
          body: StatefulBuilder(
            builder: (context, setState) {
              setParent = setState;
              return SizedBox(
                width: 240,
                height: 200,
                child: MTabs(
                  expands: true,
                  index: index,
                  onChange: (i) => setParent(() => index = i),
                  children: const [
                    MTabEntry(label: Text('A'), child: Text('pane-a')),
                    MTabEntry(label: Text('B'), child: Text('pane-b')),
                  ],
                ),
              );
            },
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    expect(find.text('pane-a'), findsOneWidget);

    await tester.tap(find.text('B'));
    await tester.pumpAndSettle();
    expect(index, 1);
    expect(find.text('pane-b'), findsOneWidget);

    setParent(() => index = 0);
    await tester.pumpAndSettle();
    expect(find.text('pane-a'), findsOneWidget);
  });

  testWidgets('vertical tab labels stack top-to-bottom', (tester) async {
    await pumpTabs(
      tester,
      direction: Axis.vertical,
      children: const [
        MTabEntry(label: Text('Top'), child: Text('pane-top')),
        MTabEntry(label: Text('Bottom'), child: Text('pane-bottom')),
      ],
    );

    expect(
      tester.getCenter(find.text('Top')).dy <
          tester.getCenter(find.text('Bottom')).dy,
      isTrue,
    );
  });

  testWidgets('expands slides selected chip between tabs', (tester) async {
    await pumpTabs(
      tester,
      children: const [
        MTabEntry(label: Text('A'), child: Text('pane-a')),
        MTabEntry(label: Text('B'), child: Text('pane-b')),
      ],
    );

    final indicator = find.byKey(const ValueKey('m-tabs-indicator'));
    expect(indicator, findsOneWidget);
    final startX = tester.getCenter(indicator).dx;

    await tester.tap(find.text('B'));
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 150));
    final midX = tester.getCenter(indicator).dx;
    expect(midX, greaterThan(startX));

    await tester.pumpAndSettle();
    expect(tester.getCenter(indicator).dx, greaterThan(midX));
  });

  /// The tab bar's border, failing readably if the bar is not a bordered box
  /// (rather than crashing on a cast).
  Border barBorder(WidgetTester tester) {
    final box = tester.widget<DecoratedBox>(find.byKey(kMTabBarKey));
    final decoration = box.decoration;
    expect(decoration, isA<BoxDecoration>());
    final border = (decoration as BoxDecoration).border;
    expect(border, isA<Border>());
    return border! as Border;
  }

  testWidgets('tab bar bottom border follows barBottomBorder', (tester) async {
    final theme = MixarThemeData.dark();
    const custom = BorderSide(color: Color(0xFF00FF00), width: 2);
    await pumpTabs(
      tester,
      barBottomBorder: custom,
      children: const [MTabEntry(label: Text('A'), child: Text('pane-a'))],
    );

    final bottom = barBorder(tester).bottom;
    expect(bottom.color, custom.color);
    // The width is forced to the theme hairline (the alignment contract), even
    // though the caller asked for 2.
    expect(bottom.width, theme.style.borderWidth);
  });

  testWidgets('tab bar falls back to the flush fill when no border is given', (
    tester,
  ) async {
    final theme = MixarThemeData.dark();
    await pumpTabs(
      tester,
      children: const [MTabEntry(label: Text('A'), child: Text('pane-a'))],
    );

    expect(barBorder(tester).bottom.color, theme.colors.muted);

    // The bar's floor height is the constant consumers align to.
    expect(tester.getSize(find.byKey(kMTabBarKey)).height, kMTabBarMinHeight);
  });
}
