import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/mixer/pad_modes.dart';
import 'package:gui_flutter/mixer/pads/hot_cue_pads.dart';
import 'package:gui_flutter/mixer/pads/key_shift_pads.dart';
import 'package:gui_flutter/mixer/pads/keyboard_pads.dart';
import 'package:gui_flutter/settings/settings_defaults.dart';
import 'package:gui_flutter/settings/settings_providers.dart';
import 'package:gui_flutter/shell/material_theme.dart';
import 'package:gui_flutter/shell/mixar_theme.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';
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
          page: kDefaultPitchPage,
          activeSemitones: active,
          onPrevPage: () {},
          onNextPage: () {},
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
      KeyShiftPads(
        page: 5,
        activeSemitones: -5,
        onPrevPage: () {},
        onNextPage: () {},
        onPress: (_) {},
      ),
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
        rootHotCue: 0,
        hotCues: const [DeckHotCue(slot: 0, positionMs: 1000)],
        onSelectRoot: (_) {},
        onPress: presses.add,
        onRelease: releases.add,
        onPrevPage: () {},
        onNextPage: () {},
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
      KeyboardPads(
        page: 3,
        rootHotCue: 0,
        hotCues: const [DeckHotCue(slot: 0, positionMs: 1000)],
        onSelectRoot: (_) {},
        onPress: (_) {},
        onRelease: (_) {},
        onPrevPage: () {},
        onNextPage: () {},
      ),
    );

    for (final label in ['-8', '-7', '-6', '-5', '-4', '-3', '-2', '-1']) {
      expect(find.text(label), findsOneWidget);
    }
  });

  testWidgets('KeyboardPads root picker lists only set hot cues', (
    tester,
  ) async {
    final selected = <int>[];
    await pumpPad(
      tester,
      KeyboardPads(
        page: kDefaultPitchPage,
        rootHotCue: 0,
        hotCues: const [
          DeckHotCue(slot: 0, positionMs: 1000),
          DeckHotCue(slot: 3, positionMs: 2000),
        ],
        onSelectRoot: selected.add,
        onPress: (_) {},
        onRelease: (_) {},
        onPrevPage: () {},
        onNextPage: () {},
      ),
    );

    // The page bar shows the current root.
    expect(find.text('HC 1'), findsOneWidget);

    await tester.tap(find.bySemanticsLabel('Keyboard root hot cue'));
    await tester.pumpAndSettle();

    // Only hot cues that are set appear in the picker.
    expect(find.textContaining('Hot cue 1'), findsOneWidget);
    expect(find.textContaining('Hot cue 4'), findsOneWidget);
    expect(find.textContaining('Hot cue 2'), findsNothing);
    expect(find.textContaining('Hot cue 3'), findsNothing);

    await tester.tap(find.textContaining('Hot cue 4'));
    await tester.pumpAndSettle();
    expect(selected, [3]);
  });

  testWidgets('KeyboardPads page bar renders and dispatches prev/next', (
    tester,
  ) async {
    final prevs = <int>[];
    final nexts = <int>[];
    var page = kDefaultPitchPage;
    await pumpPad(
      tester,
      StatefulBuilder(
        builder: (context, setState) => KeyboardPads(
          page: page,
          rootHotCue: 0,
          hotCues: const [DeckHotCue(slot: 0, positionMs: 1000)],
          onSelectRoot: (_) {},
          onPress: (_) {},
          onRelease: (_) {},
          onPrevPage: () => setState(() {
            prevs.add(page);
            page = page <= 1 ? kKeyboardPageCount : page - 1;
          }),
          onNextPage: () => setState(() {
            nexts.add(page);
            page = page >= kKeyboardPageCount ? 1 : page + 1;
          }),
        ),
      ),
    );

    expect(find.text('0…+7'), findsOneWidget);

    await tester.tap(find.byIcon(LucideIcons.chevronRight));
    await tester.pumpAndSettle();
    expect(nexts, [kDefaultPitchPage]);
    expect(find.text('-1…-8'), findsOneWidget);

    await tester.tap(find.bySemanticsLabel('Previous semitone page'));
    await tester.pumpAndSettle();
    expect(prevs, [3]);
    expect(find.text('0…+7'), findsOneWidget);
  });

  testWidgets('Keyboard page 4 shows its range and never UTIL', (tester) async {
    await pumpPad(
      tester,
      KeyboardPads(
        page: 4,
        rootHotCue: 0,
        hotCues: const [DeckHotCue(slot: 0, positionMs: 1000)],
        onSelectRoot: (_) {},
        onPress: (_) {},
        onRelease: (_) {},
        onPrevPage: () {},
        onNextPage: () {},
      ),
    );

    expect(find.text('-9…-12'), findsOneWidget);
    expect(find.text('UTIL'), findsNothing);
  });

  testWidgets('KeyShiftPads page bar renders and dispatches prev/next', (
    tester,
  ) async {
    final prevs = <int>[];
    final nexts = <int>[];
    await pumpPad(
      tester,
      KeyShiftPads(
        page: 1,
        activeSemitones: 0,
        onPrevPage: () => prevs.add(1),
        onNextPage: () => nexts.add(1),
        onPress: (_) {},
      ),
    );

    expect(find.text('+8…+12'), findsOneWidget);

    await tester.tap(find.bySemanticsLabel('Previous semitone page'));
    await tester.pump();
    await tester.tap(find.bySemanticsLabel('Next semitone page'));
    await tester.pump();
    expect(prevs, [1]);
    expect(nexts, [1]);
  });

  testWidgets('KeyShift page 5 shows UTIL', (tester) async {
    await pumpPad(
      tester,
      KeyShiftPads(
        page: 5,
        activeSemitones: 0,
        onPrevPage: () {},
        onNextPage: () {},
        onPress: (_) {},
      ),
    );

    expect(find.text('UTIL'), findsOneWidget);
  });

  testWidgets('page bar buttons do not dispatch when disabled', (tester) async {
    var prev = 0;
    var next = 0;
    await pumpPad(
      tester,
      KeyShiftPads(
        page: 1,
        activeSemitones: 0,
        disabled: true,
        onPrevPage: () => prev++,
        onNextPage: () => next++,
        onPress: (_) {},
      ),
    );

    await tester.tap(find.byIcon(LucideIcons.chevronLeft), warnIfMissed: false);
    await tester.tap(
      find.byIcon(LucideIcons.chevronRight),
      warnIfMissed: false,
    );
    await tester.pump();
    expect(prev, 0);
    expect(next, 0);
  });
}
