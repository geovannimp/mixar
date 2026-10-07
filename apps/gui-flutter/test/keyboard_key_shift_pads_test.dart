import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/mixer/pad_modes.dart';
import 'package:gui_flutter/mixer/pads/key_shift_pads.dart';
import 'package:gui_flutter/mixer/pads/keyboard_pads.dart';
import 'package:gui_flutter/settings/settings_defaults.dart';
import 'package:gui_flutter/settings/settings_providers.dart';
import 'package:gui_flutter/shell/material_theme.dart';
import 'package:gui_flutter/shell/mixar_theme.dart';
import 'package:material_ui/material_ui.dart';

import 'support/mixar_material_app.dart';

void main() {
  Future<void> pumpPad(WidgetTester tester, Widget child) async {
    final theme = MixarThemeData.dark();
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          appSettingsProvider.overrideWith((ref) async => defaultAppSettings()),
        ],
        child: MaterialApp(
          theme: materialUiThemeFromMixar(theme),
          builder: mixarMaterialAppBuilder(theme),
          home: Scaffold(body: SizedBox(width: 360, height: 320, child: child)),
        ),
      ),
    );
    await tester.pumpAndSettle();
  }

  testWidgets('KeyShiftPads dispatches slot and highlights the active offset', (
    tester,
  ) async {
    final pressed = <int>[];
    var active = 0;
    await pumpPad(
      tester,
      StatefulBuilder(
        builder: (context, setState) => KeyShiftPads(
          activeSemitones: active,
          onPress: (slot) {
            pressed.add(slot);
            final semis = kKeyShiftPadSemitones[slot];
            setState(() => active = active == semis ? 0 : semis);
          },
        ),
      ),
    );

    expect(find.byKey(const ValueKey('key-shift-pad-2')), findsOneWidget);
    expect(find.byKey(const ValueKey('key-shift-pad-2-active')), findsNothing);

    await tester.tap(find.text('+2'));
    await tester.pumpAndSettle();
    expect(pressed, [2]);
    expect(
      find.byKey(const ValueKey('key-shift-pad-2-active')),
      findsOneWidget,
    );

    await tester.tap(find.text('+2'));
    await tester.pumpAndSettle();
    expect(pressed, [2, 2]);
    expect(find.byKey(const ValueKey('key-shift-pad-2-active')), findsNothing);
  });

  testWidgets('KeyboardPads labels degrees and dispatches press/release', (
    tester,
  ) async {
    final presses = <int>[];
    final releases = <int>[];
    await pumpPad(
      tester,
      KeyboardPads(
        scale: KeyboardScale.major,
        onPress: presses.add,
        onRelease: releases.add,
      ),
    );

    for (final label in ['0', '+2', '+4', '+5', '+7', '+9', '+11', '+12']) {
      expect(find.text(label), findsOneWidget);
    }

    final gesture = await tester.startGesture(
      tester.getCenter(find.text('+4')),
    );
    await tester.pump();
    expect(presses, [2]);

    await gesture.up();
    await tester.pump();
    expect(releases, [2]);
  });

  testWidgets('KeyboardPads renders the requested scale degrees', (
    tester,
  ) async {
    await pumpPad(
      tester,
      KeyboardPads(
        scale: KeyboardScale.pentatonic,
        onPress: (_) {},
        onRelease: (_) {},
      ),
    );

    for (final label in ['0', '+2', '+4', '+7', '+9', '+12', '+14', '+16']) {
      expect(find.text(label), findsOneWidget);
    }
  });
}
