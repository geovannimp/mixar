import 'package:flutter/gestures.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/mixer/deck_pads_panel.dart';
import 'package:gui_flutter/mixer/pad_modes.dart';
import 'package:gui_flutter/mixer/pads/hot_cue_pads.dart';
import 'package:gui_flutter/mixer/pads/key_shift_pads.dart';
import 'package:gui_flutter/mixer/pads/keyboard_pads.dart';
import 'package:gui_flutter/mixer/pads/sampler_pads.dart';
import 'package:gui_flutter/settings/settings_defaults.dart';
import 'package:gui_flutter/settings/settings_providers.dart';
import 'package:gui_flutter/shell/material_theme.dart';
import 'package:gui_flutter/shell/mixar_select.dart';
import 'package:gui_flutter/shell/mixar_theme.dart';
import 'package:material_ui/material_ui.dart';
import 'package:shadcn_ui/shadcn_ui.dart';

import 'support/mixar_material_app.dart';

void main() {
  Future<void> pumpPanel(
    WidgetTester tester, {
    required bool hasTrack,
    bool disabled = false,
    PadMode padMode = PadMode.hotCue,
    List<DeckHotCue> hotCues = const [DeckHotCue(slot: 0, positionMs: 12500)],
    List<SamplerSlot> samplerSlots = const [
      SamplerSlot(label: 'Kick', durationMs: 500, path: 'demo'),
    ],
    List<SamplerBank> samplerBanks = const [
      SamplerBank(id: 'bank-1', name: 'Bank 1'),
      SamplerBank(id: 'bank-2', name: 'Bank 2', playMode: kSamplerPlayModeHold),
    ],
    String? activeBankId = 'bank-1',
    void Function(PadMode mode)? onPadMode,
    void Function(int slot, bool shift)? onHotCuePress,
    void Function(int slot)? onKeyboardPress,
    void Function(int slot)? onKeyShiftPress,
  }) async {
    var mode = padMode;
    final cues = List<DeckHotCue>.from(hotCues);
    var bankId = activeBankId;
    final theme = MixarThemeData.dark();
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          appSettingsProvider.overrideWith((ref) async => defaultAppSettings()),
        ],
        child: MaterialApp(
          theme: materialUiThemeFromMixar(theme),
          builder: mixarMaterialAppBuilder(theme),
          home: Scaffold(
            body: StatefulBuilder(
              builder: (context, setState) {
                return SizedBox(
                  width: 360,
                  height: 320,
                  child: DeckPadsPanel(
                    padMode: mode,
                    onPadMode: (next) {
                      onPadMode?.call(next);
                      setState(() => mode = next);
                    },
                    hotCues: cues,
                    onHotCuePress: (slot, shift) {
                      onHotCuePress?.call(slot, shift);
                      if (!hasTrack || disabled) {
                        return;
                      }
                      setState(() {
                        cues.removeWhere((c) => c.slot == slot);
                        cues.add(
                          DeckHotCue(slot: slot, positionMs: slot * 1000),
                        );
                      });
                    },
                    onHotCueRelease: (_) {},
                    onLoopRollPress: (_) {},
                    onLoopRollRelease: (_) {},
                    onBeatJumpPress: (_) {},
                    onBeatJumpRelease: (_) {},
                    samplerSlots: [
                      ...samplerSlots,
                      for (var i = samplerSlots.length; i < 8; i++)
                        const SamplerSlot(),
                    ],
                    samplerBanks: samplerBanks,
                    activeBankId: bankId,
                    onSamplerPress: (_, _) {},
                    onSamplerRelease: (_) {},
                    onSelectBank: (id) => setState(() => bankId = id),
                    onSaveBank: (_, _, _) {},
                    onKeyShiftPress: onKeyShiftPress ?? (_) {},
                    onKeyboardPress: onKeyboardPress ?? (_) {},
                    onKeyboardRelease: (_) {},
                    hasTrack: hasTrack,
                    disabled: disabled,
                  ),
                );
              },
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();
  }

  testWidgets('mode select switches grids', (tester) async {
    await pumpPanel(tester, hasTrack: true);

    expect(find.byType(MixarSelect<PadMode>), findsOneWidget);
    expect(find.text('Cue'), findsOneWidget);
    expect(find.text('0:12.5'), findsOneWidget);

    await tester.tap(find.byType(MixarSelect<PadMode>));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Jump').last);
    await tester.pumpAndSettle();
    expect(find.text('+1'), findsOneWidget);
    expect(find.text('0:12.5'), findsNothing);

    await tester.tap(find.byType(MixarSelect<PadMode>));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Sample').last);
    await tester.pumpAndSettle();
    expect(find.text('Bank 1'), findsOneWidget);
    expect(find.text('Kick'), findsOneWidget);
  });

  testWidgets('pad mode selector border matches panel radius', (tester) async {
    await pumpPanel(tester, hasTrack: true);
    final selectFinder = find.byType(ShadSelect<PadMode>);
    final decoration = tester
        .widget<ShadDecorator>(
          find
              .descendant(
                of: selectFinder,
                matching: find.byType(ShadDecorator),
              )
              .first,
        )
        .decoration;
    final panelBorder = tester
        .widgetList<DecoratedBox>(
          find.ancestor(of: selectFinder, matching: find.byType(DecoratedBox)),
        )
        .map((widget) => widget.decoration)
        .whereType<BoxDecoration>()
        .firstWhere((decoration) => decoration.borderRadius != null);

    final theme = MixarThemeData.dark();
    final panelSurface = Color.alphaBlend(
      theme.colors.background.withValues(alpha: 0.8),
      theme.colors.card,
    );
    final expectedBorderColor = Color.alphaBlend(
      theme.colors.border,
      panelSurface,
    );
    expect(decoration?.border?.top?.color, expectedBorderColor);
    expect(decoration?.border?.right?.color, expectedBorderColor);
    expect(decoration?.border?.bottom?.color, expectedBorderColor);
    // The left edge is the tab rail's divider; the selector must not stroke it.
    expect(decoration?.border?.toBorder().left, BorderSide.none);

    final panelRadius = panelBorder.borderRadius! as BorderRadius;
    final expectedRadius = BorderRadius.only(topRight: panelRadius.topRight);
    expect(decoration?.border?.radius, expectedRadius);
    expect(decoration?.secondaryFocusedBorder?.radius, expectedRadius);
  });

  testWidgets('hot cue press on empty slot reports the pad', (tester) async {
    final pressed = <(int, bool)>[];
    await pumpPanel(
      tester,
      hasTrack: true,
      onHotCuePress: (slot, shift) => pressed.add((slot, shift)),
    );

    await tester.tap(find.text('2'));
    await tester.pumpAndSettle();
    expect(pressed, [(1, false)]);
    expect(find.text('0:01.0'), findsOneWidget);
  });

  testWidgets('sampler bank next cycles active bank', (tester) async {
    await pumpPanel(tester, hasTrack: true);
    await tester.tap(find.byType(MixarSelect<PadMode>));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Sample').last);
    await tester.pumpAndSettle();

    await tester.tap(find.bySemanticsLabel('Next sampler bank'));
    await tester.pumpAndSettle();
    expect(find.text('Bank 2'), findsOneWidget);
    expect(find.text('hold'), findsOneWidget);
  });

  /// keep main's Keyboard / Key Shift coverage, driven through the dropdown
  testWidgets('keyboard and key shift select renders their own pad grids', (
    tester,
  ) async {
    final pressed = <int>[];
    await pumpPanel(
      tester,
      hasTrack: true,
      padMode: PadMode.keyboard,
      onKeyboardPress: pressed.add,
    );

    expect(find.byType(KeyboardPads), findsOneWidget);
    expect(find.byType(KeyShiftPads), findsNothing);
    // Default page is the `0…+7` range, with hot cue 1 showing as the root.
    expect(find.text('0…+7'), findsOneWidget);
    expect(find.text('HC 1'), findsOneWidget);

    // A pad hold forwards the pressed slot to the host.
    await tester.tap(find.text('+1'));
    await tester.pumpAndSettle();
    expect(pressed, [1]);

    await tester.tap(find.byType(MixarSelect<PadMode>));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Shift').last);
    await tester.pumpAndSettle();
    expect(find.byType(KeyShiftPads), findsOneWidget);
    expect(find.byType(KeyboardPads), findsNothing);
    expect(find.text('0…+7'), findsOneWidget);
    // The root chip belongs to the Keyboard grid only.
    expect(find.text('HC 1'), findsNothing);
  });

  testWidgets('pad mode can change without a track', (tester) async {
    await pumpPanel(tester, hasTrack: false);

    await tester.tap(find.byType(MixarSelect<PadMode>));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Roll').last);
    await tester.pumpAndSettle();
    expect(find.text('roll'), findsWidgets);
  });

  testWidgets('pad actions stay disabled without a track', (tester) async {
    await pumpPanel(tester, hasTrack: false);

    await tester.tap(find.byType(MixarSelect<PadMode>));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Roll').last);
    await tester.pumpAndSettle();
    expect(find.text('roll'), findsWidgets);
  });

  testWidgets('disabled panel blocks mode selection and pad actions', (
    tester,
  ) async {
    await pumpPanel(tester, hasTrack: true, disabled: true);

    await tester.tap(find.byType(MixarSelect<PadMode>));
    await tester.pumpAndSettle();
    expect(find.text('Jump'), findsNothing);
    expect(find.text('0:12.5'), findsOneWidget);

    await tester.tap(find.text('2'));
    await tester.pumpAndSettle();
    expect(find.text('0:01.0'), findsNothing);
  });

  testWidgets('pointer selection does not leave pad mode select focused', (
    tester,
  ) async {
    await pumpPanel(tester, hasTrack: true);
    final select = find.byType(ShadSelect<PadMode>);

    await tester.tap(select, kind: PointerDeviceKind.mouse);
    await tester.pumpAndSettle();
    await tester.tap(find.text('Roll').last, kind: PointerDeviceKind.mouse);
    await tester.pumpAndSettle();

    expect(
      tester.state<ShadSelectState<PadMode>>(select).focusNode.hasFocus,
      isFalse,
    );
  });

  testWidgets('keyboard selection keeps pad mode select focused', (
    tester,
  ) async {
    await pumpPanel(tester, hasTrack: true);
    final select = find.byType(ShadSelect<PadMode>);
    final state = tester.state<ShadSelectState<PadMode>>(select);

    state.focusNode.requestFocus();
    await tester.pump();
    await tester.sendKeyEvent(LogicalKeyboardKey.enter);
    await tester.pumpAndSettle();
    await tester.sendKeyEvent(LogicalKeyboardKey.tab);
    await tester.sendKeyEvent(LogicalKeyboardKey.enter);
    await tester.pumpAndSettle();

    expect(find.text('roll'), findsWidgets);
    expect(state.focusNode.hasFocus, isTrue);
  });
}
