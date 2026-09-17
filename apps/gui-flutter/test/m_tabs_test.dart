import 'package:flutter_test/flutter_test.dart';
import 'package:forui/forui.dart';
import 'package:gui_flutter/shell/m_tabs.dart';
import 'package:gui_flutter/shell/material_theme.dart';
import 'package:material_ui/material_ui.dart';

import 'support/forui_material_app.dart';

void main() {
  Future<void> pumpTabs(
    WidgetTester tester, {
    required List<MTabEntry> children,
    Axis direction = Axis.horizontal,
    int? index,
    ValueChanged<int>? onChange,
  }) async {
    final theme = FTheme.neutral.dark.desktop;
    await tester.pumpWidget(
      MaterialApp(
        theme: materialUiThemeFromForui(theme),
        builder: foruiMaterialAppBuilder(theme),
        home: Scaffold(
          body: SizedBox(
            width: 240,
            height: 200,
            child: MTabs(
              direction: direction,
              expands: true,
              index: index,
              onChange: onChange,
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

    final theme = FTheme.neutral.dark.desktop;
    await tester.pumpWidget(
      MaterialApp(
        theme: materialUiThemeFromForui(theme),
        builder: foruiMaterialAppBuilder(theme),
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
}
