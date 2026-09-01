import 'package:gui_flutter/shell/material_theme.dart';
import 'package:material_ui/material_ui.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:forui/forui.dart';
import 'package:gui_flutter/mixer/deck_cue_button.dart';
import 'support/forui_material_app.dart';

void main() {
  Future<void> pumpCue(
    WidgetTester tester, {
    required bool disabled,
    required VoidCallback onBegin,
    required VoidCallback onEnd,
    required VoidCallback onSet,
  }) async {
    final theme = FTheme.neutral.dark.desktop;
    await tester.pumpWidget(
      MaterialApp(
        theme: materialUiThemeFromForui(theme),
        builder: foruiMaterialAppBuilder(theme),
        home: Scaffold(
          body: Center(
            child: DeckCueButton(
              disabled: disabled,
              onBeginHold: onBegin,
              onEndHold: onEnd,
              onSetCue: onSet,
            ),
          ),
        ),
      ),
    );
  }

  testWidgets('short tap sets cue without audition', (tester) async {
    var begins = 0;
    var ends = 0;
    var sets = 0;
    await pumpCue(
      tester,
      disabled: false,
      onBegin: () => begins++,
      onEnd: () => ends++,
      onSet: () => sets++,
    );

    await tester.tap(find.text('Cue'));
    await tester.pump();
    expect(begins, 0);
    expect(ends, 0);
    expect(sets, 1);
  });

  testWidgets('hold past threshold auditions then ends', (tester) async {
    var begins = 0;
    var ends = 0;
    var sets = 0;
    await pumpCue(
      tester,
      disabled: false,
      onBegin: () => begins++,
      onEnd: () => ends++,
      onSet: () => sets++,
    );

    final gesture = await tester.startGesture(
      tester.getCenter(find.text('Cue')),
    );
    await tester.pump(const Duration(milliseconds: 200));
    expect(begins, 1);
    expect(ends, 0);
    expect(sets, 0);

    await gesture.up();
    await tester.pump();
    expect(ends, 1);
    expect(sets, 0);
  });
}
