import 'dart:typed_data';

import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/theme/deck_accent.dart';
import 'package:gui_flutter/core/theme/material_theme.dart';
import 'package:gui_flutter/core/utils/jog_ticks.dart';
import 'package:gui_flutter/widgets/mixer/jog_platter.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:material_ui/material_ui.dart';

import 'support/mixar_material_app.dart';

/// 1×1 opaque PNG — enough for the label to hold real image bytes.
final _kTinyPngArtwork = Uint8List.fromList(const [
  0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, //
  0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52,
  0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01,
  0x08, 0x02, 0x00, 0x00, 0x00, 0x90, 0x77, 0x53,
  0xDE, 0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41,
  0x54, 0x08, 0xD7, 0x63, 0xF8, 0xCF, 0xC0, 0x00,
  0x00, 0x03, 0x01, 0x01, 0x00, 0x18, 0xDD, 0x8D,
  0xB4, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E,
  0x44, 0xAE, 0x42, 0x60, 0x82,
]);

/// The platter alone, with this file's defaults. Tests that need to re-pump
/// the platter from their own state build this inside their own builder.
Widget _platterFace({
  Uint8List? artwork,
  double? bpm = 120,
  bool enabled = true,
  bool jogTouching = false,
  int positionMs = 0,
}) {
  return SizedBox(
    width: 200,
    height: 200,
    child: JogPlatter(
      accent: DeckAccent.a,
      playing: false,
      hasTrack: true,
      bpm: bpm,
      enabled: enabled,
      jogTouching: jogTouching,
      artwork: artwork,
      positionMs: positionMs,
      durationMs: 180_000,
    ),
  );
}

/// A centred [body] under the app theme, so theme plumbing lives in one place.
Widget _app(Widget body) {
  final theme = MixarThemeData.dark();
  return MaterialApp(
    theme: materialUiThemeFromMixar(theme),
    builder: mixarMaterialAppBuilder(theme),
    home: Scaffold(body: Center(child: body)),
  );
}

/// One platter on the themed scaffold.
Widget _platterApp({
  Uint8List? artwork,
  double? bpm = 120,
  bool enabled = true,
  bool jogTouching = false,
  int positionMs = 0,
}) => _app(
  _platterFace(
    artwork: artwork,
    bpm: bpm,
    enabled: enabled,
    jogTouching: jogTouching,
    positionMs: positionMs,
  ),
);

void main() {
  Future<void> pumpPlatter(
    WidgetTester tester, {
    required int positionMs,
    bool jogTouching = true,
  }) async {
    await tester.pumpWidget(
      _platterApp(positionMs: positionMs, jogTouching: jogTouching),
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
    await tester.pumpWidget(
      _app(
        StatefulBuilder(
          builder: (context, setState) {
            setPos = setState;
            // Rebuild the platter from this state, so the position jump really
            // reaches the widget.
            return _platterFace(positionMs: positionMs);
          },
        ),
      ),
    );
    await tester.pump();

    final state = tester.state(find.byType(JogPlatter));
    debugSetJogPlatterDragging(state, dragging: true);
    expect(debugReadJogPlatterDragging(state), isTrue);
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
    expect(debugReadJogPlatterDragging(stateAfter), isTrue);
    final afterPlayhead = debugReadJogPlatterRotation(stateAfter);

    expect(
      afterPlayhead,
      closeTo(duringDrag, 1),
      reason: 'local drag must freeze playhead sync',
    );
  });

  testWidgets('the centre label shows the loaded track artwork', (
    tester,
  ) async {
    Future<void> pumpWithArtwork(Uint8List? artwork) async {
      await tester.pumpWidget(_platterApp(artwork: artwork));
      await tester.pump();
    }

    await pumpWithArtwork(_kTinyPngArtwork);
    expect(
      find.byType(Image),
      findsOneWidget,
      reason: 'a loaded deck puts its cover art on the record label',
    );

    await pumpWithArtwork(null);
    expect(
      find.byType(Image),
      findsNothing,
      reason: 'without artwork the label falls back to a plain disc',
    );

    await pumpWithArtwork(Uint8List(0));
    expect(
      find.byType(Image),
      findsNothing,
      reason: 'empty artwork bytes must not reach the decoder',
    );
  });
}
