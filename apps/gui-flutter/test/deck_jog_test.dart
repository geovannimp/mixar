import 'package:flutter/widgets.dart';
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
    bool jogTouching = true,
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
                jogTouching: jogTouching,
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
    final startRot = debugReadJogPlatterRotation(
      tester.state(find.byType(JogPlatter)),
    );

    await pumpPlatter(tester, positionMs: movedMs);
    await tester.pump();
    final endRot = debugReadJogPlatterRotation(
      tester.state(find.byType(JogPlatter)),
    );

    expect(
      endRot - startRot,
      closeTo(45, 1),
      reason:
          'hardware touch must rotate one visual rev per vinyl rev of music',
    );
  });

  testWidgets('JogPlatter completes a full turn for one vinyl revolution', (
    tester,
  ) async {
    await pumpPlatter(tester, positionMs: 0);
    await tester.pump();
    final startRot = debugReadJogPlatterRotation(
      tester.state(find.byType(JogPlatter)),
    );

    await pumpPlatter(tester, positionMs: vinylRevolutionMs().round());
    await tester.pump();
    final endRot = debugReadJogPlatterRotation(
      tester.state(find.byType(JogPlatter)),
    );

    expect(endRot - startRot, closeTo(360, 1));
  });

  testWidgets('local drag freezes playhead-driven rotation', (tester) async {
    var positionMs = 0;
    late StateSetter setPos;
    final theme = MixarThemeData.dark();
    await tester.pumpWidget(
      MaterialApp(
        theme: materialUiThemeFromMixar(theme),
        builder: mixarMaterialAppBuilder(theme),
        home: Scaffold(
          body: StatefulBuilder(
            builder: (context, setState) {
              setPos = setState;
              return Center(
                child: SizedBox(
                  width: 200,
                  height: 200,
                  child: JogPlatter(
                    accent: FaderAccent.a,
                    playing: false,
                    hasTrack: true,
                    bpm: 120,
                    enabled: true,
                    jogTouching: false,
                    positionMs: positionMs,
                    durationMs: 180_000,
                  ),
                ),
              );
            },
          ),
        ),
      ),
    );
    await tester.pump();

    final state = tester.state(find.byType(JogPlatter));
    (state as dynamic).debugSetDragging(true);
    expect((state as dynamic).debugDragging, isTrue);
    final duringDrag = debugReadJogPlatterRotation(state);

    // Playhead jumps a full vinyl rev; active drag must keep the local angle.
    setPos(() => positionMs = vinylRevolutionMs().round());
    await tester.pump();
    final stateAfter = tester.state(find.byType(JogPlatter));
    expect(
      identical(state, stateAfter),
      isTrue,
      reason: 'JogPlatter State must be preserved across position updates',
    );
    expect((stateAfter as dynamic).debugDragging, isTrue);
    final afterPlayhead = debugReadJogPlatterRotation(stateAfter);

    expect(
      afterPlayhead,
      closeTo(duringDrag, 1),
      reason: 'local drag must freeze playhead sync',
    );
  });
}
