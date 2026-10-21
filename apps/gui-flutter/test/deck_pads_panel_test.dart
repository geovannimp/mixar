import 'package:flutter/gestures.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/models/pad_modes.dart';
import 'package:gui_flutter/core/theme/material_theme.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/providers/settings/settings_providers.dart';
import 'package:gui_flutter/src/rust/api/engine.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';
import 'package:gui_flutter/core/theme/mixar_button_style.dart';
import 'package:gui_flutter/widgets/mixer/deck_pads_panel.dart';
import 'package:gui_flutter/widgets/mixer/pads/pad_button.dart';
import 'package:gui_flutter/widgets/mixer/mixer_button.dart';
import 'package:gui_flutter/widgets/mixer/pads/hot_cue_pads.dart';
import 'package:gui_flutter/widgets/mixer/pads/key_shift_pads.dart';
import 'package:gui_flutter/widgets/mixer/pads/keyboard_pads.dart';
import 'package:gui_flutter/widgets/mixer/pads/sampler_pads.dart';
import 'package:gui_flutter/widgets/shell/mixar_select.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:material_ui/material_ui.dart';
import 'package:shadcn_ui/shadcn_ui.dart';

import 'support/mixar_material_app.dart';
import 'support/pad_finders.dart';

Finder padLabel(String text) => find.text(text, findRichText: true);

void main() {
  Future<void> pumpPanel(
    WidgetTester tester, {
    required bool hasTrack,
    bool disabled = false,
    SelectStyleSetting selectStyle = SelectStyleSetting.auto,
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
    bool stemsReady = false,
    bool stemsLive = false,
    bool realtimeStemsEnabled = false,
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
          appSettingsProvider.overrideWith(
            (ref) async =>
                copyAppSettings(defaultAppSettings(), selectStyle: selectStyle),
          ),
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
                    onCreateBank: (_, _) {},
                    onSelectPage: (_) {},
                    onKeyShiftPress: onKeyShiftPress ?? (_) {},
                    onKeyboardPress: onKeyboardPress ?? (_) {},
                    onKeyboardRelease: (_) {},
                    stemsReady: stemsReady,
                    stemsLive: stemsLive,
                    realtimeStemsEnabled: realtimeStemsEnabled,
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
    expect(padLabel('Sample (Bank 1)'), findsOneWidget);
    final sampleRich = tester.widget<RichText>(padLabel('Sample (Bank 1)'));
    final sampleInner =
        (sampleRich.text as TextSpan).children!.single as TextSpan;
    final bankSuffix = sampleInner.children!.cast<TextSpan>().singleWhere(
      (span) => span.text == ' (Bank 1)',
    );
    expect(
      bankSuffix.style?.color,
      MixarThemeData.dark().colors.mutedForeground,
    );
    expect(find.text('Kick'), findsOneWidget);
  });

  testWidgets('stems mode shows its source only when realtime is enabled', (
    tester,
  ) async {
    await pumpPanel(
      tester,
      hasTrack: true,
      padMode: PadMode.stems,
      stemsReady: true,
      stemsLive: true,
      realtimeStemsEnabled: true,
    );
    expect(padLabel('Stems (Realtime)'), findsOneWidget);
    final rich = tester.widget<RichText>(padLabel('Stems (Realtime)'));
    // ignore: avoid_print
    final inner = (rich.text as TextSpan).children!.single as TextSpan;
    final suffix = inner.children!.cast<TextSpan>().singleWhere(
      (span) => span.text == ' (Realtime)',
    );
    expect(suffix.style?.color, MixarThemeData.dark().colors.mutedForeground);

    // The open list uses plain mode names; only the collapsed chip carries
    // the selected mode's source suffix.
    await tester.tap(find.byType(MixarSelect<PadMode>));
    await tester.pumpAndSettle();
    expect(find.text('Stems'), findsOneWidget);
    expect(padLabel('Stems (Realtime)'), findsOneWidget);
    await tester.tap(find.text('Stems'));
    await tester.pumpAndSettle();

    await pumpPanel(
      tester,
      hasTrack: true,
      padMode: PadMode.stems,
      stemsReady: true,
      realtimeStemsEnabled: true,
    );
    expect(padLabel('Stems (Offline)'), findsOneWidget);

    await pumpPanel(
      tester,
      hasTrack: true,
      padMode: PadMode.stems,
      stemsReady: true,
      stemsLive: true,
      realtimeStemsEnabled: false,
    );
    expect(find.text('Stems'), findsOneWidget);
    expect(padLabel('Stems (Realtime)'), findsNothing);
  });

  testWidgets('mobile pad mode select shows Pad mode title', (tester) async {
    await pumpPanel(
      tester,
      hasTrack: true,
      selectStyle: SelectStyleSetting.mobile,
    );

    await tester.tap(find.byType(MixarSelect<PadMode>));
    await tester.pumpAndSettle();
    expect(find.text('Pad mode'), findsOneWidget);
    expect(find.text('Jump'), findsOneWidget);
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

  testWidgets('sampler config menu switches banks', (tester) async {
    await pumpPanel(tester, hasTrack: true);
    await tester.tap(find.byType(MixarSelect<PadMode>));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Sample').last);
    await tester.pumpAndSettle();
    expect(padLabel('Sample (Bank 1)'), findsOneWidget);

    final configButton = find.byType(MixerButton);
    expect(
      tester.widget<MixerButton>(configButton).variant,
      MixarButtonVariant.outline,
    );

    await tester.tap(find.bySemanticsLabel('Sample settings'));
    await tester.pumpAndSettle();
    expect(
      tester.widget<MixerButton>(configButton).variant,
      MixarButtonVariant.primary,
    );
    expect(find.text('New bank…'), findsOneWidget);
    expect(find.text('Edit bank…'), findsOneWidget);

    await tester.tap(find.text('Bank 2'));
    await tester.pumpAndSettle();
    expect(padLabel('Sample (Bank 2)'), findsOneWidget);
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
    // Default page range is part of the selected mode label.
    expect(padLabel('Keys (0…+7)'), findsOneWidget);

    // A pad hold forwards the pressed slot to the host.
    await tester.tap(find.text('+1'));
    await tester.pumpAndSettle();
    expect(pressed, [1]);

    // Root cue and page live in the config menu.
    await tester.tap(find.bySemanticsLabel('Keys settings'));
    await tester.pumpAndSettle();
    expect(find.text('Root hot cue'), findsOneWidget);
    expect(find.text('Hot cue 1'), findsOneWidget);
    expect(find.text('-1…-8'), findsOneWidget);
    await tester.tapAt(const Offset(4, 4));
    await tester.pumpAndSettle();

    await tester.tap(find.byType(MixarSelect<PadMode>));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Shift').last);
    await tester.pumpAndSettle();
    expect(find.byType(KeyShiftPads), findsOneWidget);
    expect(find.byType(KeyboardPads), findsNothing);
    expect(padLabel('Shift (0…+7)'), findsOneWidget);
  });

  testWidgets('pad mode can change without a track', (tester) async {
    await pumpPanel(tester, hasTrack: false);

    await tester.tap(find.byType(MixarSelect<PadMode>));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Roll').last);
    await tester.pumpAndSettle();
    expect(find.text('ROLL'), findsWidgets);
  });

  testWidgets('momentary pads highlight primary while held', (tester) async {
    await pumpPanel(tester, hasTrack: true);
    await tester.tap(find.byType(MixarSelect<PadMode>));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Jump').last);
    await tester.pumpAndSettle();

    final theme = MixarThemeData.dark();
    final pad = find.byType(PadButton).first;
    expect(padDecorationOf(tester, pad).color, theme.colors.card);
    final gesture = await tester.startGesture(tester.getCenter(pad));
    await tester.pump();
    expect(padDecorationOf(tester, pad).color, theme.colors.primary);
    expect(
      padDecorationOf(tester, pad).border!.top.color,
      theme.colors.primary,
    );

    await gesture.up();
    await tester.pump();
    expect(padDecorationOf(tester, pad).color, theme.colors.card);
  });

  testWidgets('new bank dialog opens from the config menu', (tester) async {
    await pumpPanel(tester, hasTrack: true);
    await tester.tap(find.byType(MixarSelect<PadMode>));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Sample').last);
    await tester.pumpAndSettle();

    await tester.tap(find.bySemanticsLabel('Sample settings'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('New bank…'));
    await tester.pumpAndSettle();
    expect(find.text('New bank'), findsOneWidget);

    await tester.tap(find.text('Cancel'));
    await tester.pumpAndSettle();
  });

  testWidgets('config button keeps clear of the bottom-right pad', (
    tester,
  ) async {
    await pumpPanel(tester, hasTrack: true);
    await tester.tap(find.byType(MixarSelect<PadMode>));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Sample').last);
    await tester.pumpAndSettle();

    final pad = tester.getRect(find.byType(PadButton).last);
    final button = tester.getRect(find.bySemanticsLabel('Sample settings'));
    expect(pad.bottom, lessThanOrEqualTo(button.top));
  });

  testWidgets('sampler duration sits bottom-right like a cue time', (
    tester,
  ) async {
    await pumpPanel(tester, hasTrack: true);
    await tester.tap(find.byType(MixarSelect<PadMode>));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Sample').last);
    await tester.pumpAndSettle();

    final padRect = tester.getRect(find.byType(PadButton).first);
    final time = tester.getRect(find.text('0:00.5'));
    expect(time.right, closeTo(padRect.right - 8, 0.5));
    expect(time.bottom, closeTo(padRect.bottom - 8, 0.5));
    // The centred name stays above the footer.
    expect(
      tester.getRect(find.text('Kick')).bottom,
      lessThanOrEqualTo(time.top),
    );
  });

  testWidgets('assigned cue rests outlined and fills while held', (
    tester,
  ) async {
    await pumpPanel(tester, hasTrack: true);
    final theme = MixarThemeData.dark();
    final pad = find.byType(PadButton).first;
    BoxDecoration decoration() =>
        tester
                .widget<DecoratedBox>(
                  find
                      .descendant(of: pad, matching: find.byType(DecoratedBox))
                      .first,
                )
                .decoration
            as BoxDecoration;

    expect(decoration().color, theme.colors.card);
    expect(
      decoration().border!.top.color,
      Color.alphaBlend(
        hotCueAccent(0).fill.withValues(alpha: 0.4),
        theme.colors.border,
      ),
    );

    final gesture = await tester.startGesture(tester.getCenter(pad));
    await tester.pump();
    expect(decoration().color, hotCueAccent(0).fill.withValues(alpha: 1));

    await gesture.up();
    await tester.pump();
    expect(decoration().color, theme.colors.card);
  });

  testWidgets('pad actions stay disabled without a track', (tester) async {
    await pumpPanel(tester, hasTrack: false);

    await tester.tap(find.byType(MixarSelect<PadMode>));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Roll').last);
    await tester.pumpAndSettle();
    expect(find.text('ROLL'), findsWidgets);
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

    expect(find.text('ROLL'), findsWidgets);
    expect(state.focusNode.hasFocus, isTrue);
  });
}
