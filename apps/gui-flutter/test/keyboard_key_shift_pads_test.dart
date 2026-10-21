import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/models/pad_modes.dart';
import 'package:gui_flutter/core/theme/material_theme.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/providers/settings/settings_providers.dart';
import 'package:gui_flutter/widgets/mixer/pads/key_shift_pads.dart';
import 'package:gui_flutter/widgets/mixer/pads/keyboard_pads.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:material_ui/material_ui.dart';

import 'support/mixar_material_app.dart';
import 'support/pad_finders.dart';

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
          page: kDefaultPitchPage,
          activeSemitones: active,
          onPress: (slot) {
            pressed.add(slot);
            final semis = keyShiftPage(kDefaultPitchPage)[slot].semitones;
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
    final activeDecoration = padDecorationOf(
      tester,
      find.byKey(const ValueKey('key-shift-pad-2-active')),
    );
    // Single-select stays outline-only: card fill + accent stroke.
    expect(activeDecoration.color, MixarThemeData.dark().colors.card);

    await tester.tap(find.text('+2'));
    await tester.pumpAndSettle();
    expect(pressed, [2, 2]);
    expect(find.byKey(const ValueKey('key-shift-pad-2-active')), findsNothing);
  });

  testWidgets('KeyShiftPads renders page-5 specials and highlights semitones', (
    tester,
  ) async {
    await pumpPad(
      tester,
      KeyShiftPads(page: 5, activeSemitones: -5, onPress: (_) {}),
    );

    for (final label in [
      'RESET',
      'DOWN',
      '-5',
      '-12',
      'SYNC',
      'UP',
      '+7',
      '+12',
    ]) {
      expect(find.text(label), findsOneWidget);
    }
    // Slot 2 is an absolute -5 semitone; the page-5 specials never highlight.
    expect(
      find.byKey(const ValueKey('key-shift-pad-2-active')),
      findsOneWidget,
    );
    for (final slot in [0, 1, 4, 5]) {
      expect(find.byKey(ValueKey('key-shift-pad-$slot-active')), findsNothing);
    }
  });

  testWidgets('KeyboardPads labels page pads and dispatches press/release', (
    tester,
  ) async {
    final presses = <int>[];
    final releases = <int>[];
    await pumpPad(
      tester,
      KeyboardPads(
        page: kDefaultPitchPage,
        onPress: presses.add,
        onRelease: releases.add,
      ),
    );

    for (final label in ['0', '+1', '+2', '+3', '+4', '+5', '+6', '+7']) {
      expect(find.text(label), findsOneWidget);
    }

    final gesture = await tester.startGesture(
      tester.getCenter(find.text('+4')),
    );
    await tester.pump();
    expect(presses, [4]);

    await gesture.up();
    await tester.pump();
    expect(releases, [4]);
  });

  testWidgets('KeyboardPads renders the requested page', (tester) async {
    await pumpPad(
      tester,
      KeyboardPads(page: 3, onPress: (_) {}, onRelease: (_) {}),
    );

    for (final label in ['-8', '-7', '-6', '-5', '-4', '-3', '-2', '-1']) {
      expect(find.text(label), findsOneWidget);
    }
  });
}
