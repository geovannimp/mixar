import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/theme/material_theme.dart';
import 'package:gui_flutter/widgets/shell/m_card.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:material_ui/material_ui.dart';

import 'support/mixar_material_app.dart';

const _override = Color(0xFF2DD4BF);

Widget _mixarApp(MixarThemeData base, Widget body) => MaterialApp(
  theme: materialUiThemeFromMixar(base),
  builder: mixarMaterialAppBuilder(base),
  home: Scaffold(body: body),
);

void main() {
  testWidgets('MCard paints card fill + square border', (tester) async {
    final base = MixarThemeData.dark();
    await tester.pumpWidget(
      _mixarApp(base, const MCard(child: SizedBox(width: 40, height: 40))),
    );

    final decorated = tester.widget<DecoratedBox>(
      find.descendant(
        of: find.byType(MCard),
        matching: find.byType(DecoratedBox),
      ),
    );
    final decoration = decorated.decoration as BoxDecoration;
    expect(decoration.color, base.colors.card);
    expect(decoration.borderRadius, base.style.borderRadius.lg);
    expect(
      decoration.border,
      Border.all(color: base.colors.border, width: base.style.borderWidth),
    );
    expect(find.byType(ClipRRect), findsNothing);
  });

  testWidgets('MCard can paint only the horizontal hairlines', (tester) async {
    final base = MixarThemeData.dark();
    await tester.pumpWidget(
      _mixarApp(
        base,
        const MCard(sideBorders: false, child: SizedBox(width: 40, height: 40)),
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
      Border.symmetric(
        horizontal: BorderSide(
          color: base.colors.border,
          width: base.style.borderWidth,
        ),
      ),
    );
  });

  testWidgets('a border color override closes every edge', (tester) async {
    final base = MixarThemeData.dark();
    await tester.pumpWidget(
      _mixarApp(
        base,
        const MCard(
          sideBorders: false,
          topBorder: false,
          borderColor: _override,
          child: SizedBox(width: 40, height: 40),
        ),
      ),
    );

    final decorated = tester.widget<DecoratedBox>(find.byKey(MCard.outlineKey));
    final decoration = decorated.decoration as BoxDecoration;
    expect(
      decoration.border,
      Border.all(color: _override, width: base.style.borderWidth),
      reason: 'interaction chrome is never an open outline',
    );
  });

  testWidgets('MCard clips with ClipRRect when requested', (tester) async {
    final base = MixarThemeData.dark();
    await tester.pumpWidget(
      _mixarApp(
        base,
        const MCard(
          clipBehavior: Clip.antiAlias,
          child: SizedBox(width: 40, height: 40),
        ),
      ),
    );

    expect(find.byType(ClipRRect), findsOneWidget);
  });

  testWidgets('MCard accepts a border color override', (tester) async {
    final base = MixarThemeData.dark();
    await tester.pumpWidget(
      _mixarApp(
        base,
        const MCard(
          borderColor: _override,
          child: SizedBox(width: 40, height: 40),
        ),
      ),
    );

    final decorated = tester.widget<DecoratedBox>(find.byKey(MCard.outlineKey));
    final decoration = decorated.decoration as BoxDecoration;
    expect(
      decoration.border,
      Border.all(color: _override, width: base.style.borderWidth),
    );
  });

  testWidgets('MCard insets children inside the hairline border', (
    tester,
  ) async {
    final base = MixarThemeData.dark();
    const childKey = ValueKey('m-card-child');
    await tester.pumpWidget(
      _mixarApp(
        base,
        const Align(
          alignment: Alignment.topLeft,
          child: MCard(child: SizedBox(key: childKey, width: 40, height: 40)),
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

  testWidgets('an outline inset keeps the closed edge inside the card', (
    tester,
  ) async {
    final base = MixarThemeData.dark();
    await tester.pumpWidget(
      _mixarApp(
        base,
        const Align(
          alignment: Alignment.topLeft,
          child: MCard(
            borderColor: _override,
            outlineInset: EdgeInsets.symmetric(horizontal: 1),
            child: SizedBox(width: 40, height: 40),
          ),
        ),
      ),
    );

    final cardRect = tester.getRect(find.byType(MCard));
    final outlineRect = tester.getRect(find.byKey(MCard.outlineKey));
    expect(outlineRect.left, cardRect.left + 1);
    expect(outlineRect.right, cardRect.right - 1);
    expect(outlineRect.top, cardRect.top);
    expect(outlineRect.bottom, cardRect.bottom);
  });

  testWidgets('an outline inset clamps to the card hairline', (tester) async {
    final base = MixarThemeData.dark();
    final borderW = base.style.borderWidth;
    await tester.pumpWidget(
      _mixarApp(
        base,
        const Align(
          alignment: Alignment.topLeft,
          child: MCard(
            borderColor: _override,
            // Negative and over-large sides stay on the card: the outline may
            // use the hairline ring and no more.
            outlineInset: EdgeInsets.fromLTRB(-4, 4, 40, 0),
            child: SizedBox(width: 40, height: 40),
          ),
        ),
      ),
    );

    final cardRect = tester.getRect(find.byType(MCard));
    final outlineRect = tester.getRect(find.byKey(MCard.outlineKey));
    expect(outlineRect.left, cardRect.left);
    expect(outlineRect.top, cardRect.top + borderW);
    expect(outlineRect.right, cardRect.right - borderW);
    expect(outlineRect.bottom, cardRect.bottom);
  });

  testWidgets('toggling the outline does not remount the card body', (
    tester,
  ) async {
    final base = MixarThemeData.dark();

    Widget build({Color? border}) => _mixarApp(
      base,
      MCard(
        borderColor: border,
        outlineInset: const EdgeInsets.symmetric(horizontal: 1),
        child: const _Counter(),
      ),
    );

    await tester.pumpWidget(build());
    final state = tester.state<_CounterState>(find.byType(_Counter));
    await tester.tap(find.byType(_Counter));
    await tester.pump();
    expect(state.value, 1);

    await tester.pumpWidget(build(border: _override));
    expect(tester.state<_CounterState>(find.byType(_Counter)), same(state));
    expect(state.value, 1, reason: 'hover chrome must not reset card state');

    await tester.pumpWidget(build());
    expect(tester.state<_CounterState>(find.byType(_Counter)), same(state));
    expect(state.value, 1);
  });

  testWidgets('a tight box keeps the card fill under the outline', (
    tester,
  ) async {
    final base = MixarThemeData.dark();
    await tester.pumpWidget(
      _mixarApp(
        base,
        const Center(
          child: SizedBox(
            width: 100,
            height: 60,
            child: MCard(
              borderColor: _override,
              outlineInset: EdgeInsets.symmetric(horizontal: 1),
              child: SizedBox(width: 10, height: 10),
            ),
          ),
        ),
      ),
    );

    expect(tester.getSize(find.byType(MCard)), const Size(100, 60));
    final fill = tester.getRect(
      find.descendant(
        of: find.byType(MCard),
        matching: find.byWidgetPredicate(
          (widget) =>
              widget is DecoratedBox &&
              widget.decoration is BoxDecoration &&
              (widget.decoration as BoxDecoration).color == base.colors.card,
        ),
      ),
    );
    expect(
      fill.size,
      const Size(100, 60),
      reason: 'the card fill must span the box the outline fills',
    );
    final outline = tester.getRect(find.byKey(MCard.outlineKey));
    expect(outline.size, const Size(98, 60));
  });
}

class _Counter extends StatefulWidget {
  const new();

  @override
  State<_Counter> createState() => _CounterState();
}

class _CounterState extends State<_Counter> {
  int value = 0;

  @override
  Widget build(BuildContext context) => GestureDetector(
    behavior: HitTestBehavior.opaque,
    onTap: () => setState(() => value++),
    child: const SizedBox(width: 40, height: 40),
  );
}
