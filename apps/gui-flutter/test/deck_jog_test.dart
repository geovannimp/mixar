import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/mixer/deck_jog.dart';
import 'package:gui_flutter/mixer/fader_slider.dart';
import 'package:gui_flutter/mixer/jog_ticks.dart';
import 'package:gui_flutter/shell/material_theme.dart';
import 'package:gui_flutter/shell/mixar_theme.dart';
import 'package:material_ui/material_ui.dart';

import 'support/mixar_material_app.dart';

void main() {
  Future<void> pumpPlatter(
    WidgetTester tester, {
    required int positionMs,
  }) async {
    final theme = MixarThemeData.dark();
    await tester.pumpWidget(
      MaterialApp(
        theme: materialUiThemeFromMixar(theme),
        builder: mixarMaterialAppBuilder(theme),
        home: Scaffold(
          body: Center(
            child: SizedBox(
              width: 200,
              height: 200,
              child: JogPlatter(
                accent: FaderAccent.a,
                playing: false,
                hasTrack: true,
                bpm: 120,
                enabled: true,
                jogTouching: true,
                positionMs: positionMs,
                durationMs: 180_000,
              ),
            ),
          ),
        ),
      ),
    );
  }

  testWidgets('JogPlatter follows playhead on vinyl timebase while touching', (
    tester,
  ) async {
    // One-eighth of a vinyl revolution → +45°.
    final movedMs = (vinylRevolutionMs() / 8).round();

    await pumpPlatter(tester, positionMs: 0);
    await tester.pump();
    final state = tester.state(find.byType(JogPlatter)) as dynamic;
    final startRot = state.debugRotationDeg as double;

    await pumpPlatter(tester, positionMs: movedMs);
    await tester.pump();
    final endRot = state.debugRotationDeg as double;

    expect(
      endRot - startRot,
      closeTo(45, 1),
      reason: 'hardware touch must rotate one visual rev per vinyl rev of music',
    );
  });

  testWidgets('JogPlatter completes a full turn for one vinyl revolution', (
    tester,
  ) async {
    await pumpPlatter(tester, positionMs: 0);
    await tester.pump();
    final state = tester.state(find.byType(JogPlatter)) as dynamic;
    final startRot = state.debugRotationDeg as double;

    await pumpPlatter(tester, positionMs: vinylRevolutionMs().round());
    await tester.pump();
    final endRot = state.debugRotationDeg as double;

    expect(endRot - startRot, closeTo(360, 1));
  });
}
