import 'dart:io';
import 'dart:typed_data';

import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/theme/material_theme.dart';
import 'package:gui_flutter/widgets/mixer/pads/beat_jump_pads.dart';
import 'package:gui_flutter/widgets/mixer/pads/hot_cue_pads.dart';
import 'package:gui_flutter/widgets/mixer/pads/key_shift_pads.dart';
import 'package:gui_flutter/widgets/mixer/pads/keyboard_pads.dart';
import 'package:gui_flutter/widgets/mixer/pads/loop_roll_pads.dart';
import 'package:gui_flutter/widgets/mixer/pads/sampler_pads.dart';
import 'package:gui_flutter/widgets/mixer/pads/stems_pads.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:material_ui/material_ui.dart';

import 'support/mixar_material_app.dart';

void main() {
  Future<void> loadFont(String family, String path) async {
    final loader = FontLoader(family)
      ..addFont(
        Future.value(ByteData.sublistView(File(path).readAsBytesSync())),
      );
    await loader.load();
  }

  testWidgets('every caption renders at one size', (tester) async {
    await loadFont('Outfit', 'fonts/Outfit-Variable.ttf');
    await loadFont('Space Grotesk', 'fonts/SpaceGrotesk-Variable.ttf');
    await loadFont('Noto Sans Mono', 'fonts/NotoSansMono-Variable.ttf');

    final theme = MixarThemeData.dark();
    final modes = <(String, Widget, String)>[
      (
        'hotcue',
        HotCuePads(
          hotCues: const [DeckHotCue(slot: 0, positionMs: 1500)],
          onPress: (_, _) {},
          onRelease: (_) {},
        ),
        'CUE',
      ),
      ('roll', LoopRollPads(onPress: (_) {}, onRelease: (_) {}), 'ROLL'),
      ('beat', BeatJumpPads(onPress: (_) {}, onRelease: (_) {}), 'BEAT'),
      (
        'sampler',
        SamplerPads(
          slots: const [SamplerSlot(label: 'Kick', durationMs: 1200)],
          holdLike: true,
          onPress: (_, _) {},
          onRelease: (_) {},
        ),
        'SAMPLE',
      ),
      (
        'keyshift',
        KeyShiftPads(page: 0, activeSemitones: 3, onPress: (_) {}),
        'SHIFT',
      ),
      (
        'keyboard',
        KeyboardPads(page: 0, onPress: (_) {}, onRelease: (_) {}),
        'NOTE',
      ),
      (
        'stems',
        StemsPads(
          stemMute: const [true, false, false, false],
          stemIsolate: 2,
          stemsReady: true,
          stemsGenerating: false,
          onPress: (_) {},
        ),
        'MUTE',
      ),
    ];

    final heights = <String, double>{};
    for (final (name, mode, caption) in modes) {
      await tester.pumpWidget(
        ProviderScope(
          child: MaterialApp(
            theme: materialUiThemeFromMixar(theme),
            builder: mixarMaterialAppBuilder(theme),
            home: Scaffold(
              backgroundColor: theme.colors.background,
              body: Center(
                child: SizedBox(width: 300, height: 240, child: mode),
              ),
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();
      heights[name] = tester.getRect(find.text(caption).first).height;
    }

    expect(heights.values.toSet(), hasLength(1), reason: '$heights');
  });
}
