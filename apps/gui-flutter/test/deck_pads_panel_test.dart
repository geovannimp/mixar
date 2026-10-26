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

TextSpan padSuffixSpan(WidgetTester tester, String label, String suffixText) {
  final rich = tester.widget<RichText>(padLabel(label));
  final inner = (rich.text as TextSpan).children!.single as TextSpan;
  return inner.children!.cast<TextSpan>().singleWhere(
    (span) => span.text == suffixText,
  );
}

/// The pad-mode suffix must render in the muted foreground colour.
void expectMutedPadSuffix(
  WidgetTester tester,
  String label,
  String suffixText,
) {
  expect(
    padSuffixSpan(tester, label, suffixText).style?.color,
    MixarThemeData.dark().colors.mutedForeground,
  );
}

void main() {
  Future<void> pumpPanel(
    WidgetTester tester, {
    required bool hasTrack,
    bool disabled = false,
    bool bordered = true,
    Size size = const Size(360, 320),
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
                  width: size.width,
                  height: size.height,
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
                    bordered: bordered,
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

  testWidgets('compact cue pads remain usable without a config button', (
    tester,
  ) async {
    final pressed = <int>[];
    await pumpPanel(
      tester,
      hasTrack: true,
      size: const Size(200, 70),
      hotCues: const [],
      onHotCuePress: (slot, _) => pressed.add(slot),
    );
    expect(tester.takeException(), isNull);
    await tester.sendEventToBinding(
      PointerScrollEvent(
        position: tester.getCenter(find.byType(HotCuePads)),
        scrollDelta: const Offset(0, 100),
      ),
    );
    await tester.pumpAndSettle();
    final scroller = tester.state<ScrollableState>(
      find.descendant(
        of: find.byType(HotCuePads),
        matching: find.byType(Scrollable),
      ),
    );
    expect(scroller.position.pixels, greaterThan(0));
    expect(pressed, isEmpty, reason: 'scrolling must not trigger a pad');
    await tester.ensureVisible(find.text('8'));
    await tester.tap(find.text('8'));
    await tester.pumpAndSettle();
    expect(pressed, [7]);
    expect(find.text('0:07.0'), findsOneWidget);
    expect(tester.takeException(), isNull);
  });

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
    expectMutedPadSuffix(tester, 'Sample (Bank 1)', ' (Bank 1)');
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
    expectMutedPadSuffix(tester, 'Stems (Realtime)', ' (Realtime)');

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

  testWidgets('header divider shares the card frame border color', (
    tester,
  ) async {
    await pumpPanel(tester, hasTrack: true);
    final selectFinder = find.byType(ShadSelect<PadMode>);
    final selectDecoration = tester
        .widget<ShadDecorator>(
          find
              .descendant(
                of: selectFinder,
                matching: find.byType(ShadDecorator),
              )
              .first,
        )
        .decoration;
    final ancestorDecorations = tester
        .widgetList<DecoratedBox>(
          find.ancestor(of: selectFinder, matching: find.byType(DecoratedBox)),
        )
        .map((widget) => widget.decoration)
        .whereType<BoxDecoration>()
        .toList();
    final theme = MixarThemeData.dark();

    // The selector paints no stroke: a second, differently blended line over
    // the divider is what made the header border look off color.
    final stroke = selectDecoration?.border?.toBorder();
    expect(stroke, isNotNull);
    expect([
      stroke!.top,
      stroke.right,
      stroke.bottom,
      stroke.left,
    ], everyElement(BorderSide.none));

    // Header divider and card frame both come from the raw border token.
    final divider = ancestorDecorations.firstWhere(
      (decoration) =>
          decoration.border?.bottom != BorderSide.none &&
          decoration.border?.top == BorderSide.none,
    );
    expect(divider.border!.bottom.color, theme.colors.border);

    final frame = ancestorDecorations.firstWhere(
      (decoration) =>
          decoration.borderRadius != null && decoration.border != null,
    );
    final frameBorder = frame.border! as Border;
    expect(frameBorder.top.color, theme.colors.border);
    expect(frameBorder.right.color, theme.colors.border);
    expect(frameBorder.bottom.color, theme.colors.border);

    // The focus ring still follows the card's top-right corner.
    final expectedRadius = BorderRadius.only(
      topRight: (frame.borderRadius! as BorderRadius).topRight,
    );
    expect(selectDecoration?.secondaryFocusedBorder?.radius, expectedRadius);
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
    final button = tester.widget<MixerButton>(configButton);
    expect(button.variant, MixarButtonVariant.ghost);
    expect(button.selected, isFalse);

    await tester.tap(find.bySemanticsLabel('Sample settings'));
    await tester.pumpAndSettle();
    expect(tester.widget<MixerButton>(configButton).selected, isTrue);
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

  testWidgets('configurable modes fit the compact pane header', (tester) async {
    await pumpPanel(
      tester,
      hasTrack: true,
      size: const Size(200, 70),
      padMode: PadMode.keyShift,
    );
    expect(tester.takeException(), isNull);
    expect(
      tester.getRect(find.bySemanticsLabel('Shift settings')).width,
      greaterThan(0),
    );
  });

  testWidgets('config cell clears the divider in a tabs-hosted panel', (
    tester,
  ) async {
    await pumpPanel(
      tester,
      hasTrack: true,
      bordered: false,
      padMode: PadMode.sampler,
    );
    final button = tester.getRect(find.bySemanticsLabel('Sample settings'));
    final select = tester.getRect(find.byType(MixarSelect<PadMode>));
    final theme = MixarThemeData.dark();
    // No card frame to clear, but the header divider must still show.
    expect(button.top, select.top);
    expect(button.bottom, select.bottom - theme.style.borderWidth);
  });

  testWidgets('config button sits in the header above every pad', (
    tester,
  ) async {
    await pumpPanel(tester, hasTrack: true);
    await tester.tap(find.byType(MixarSelect<PadMode>));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Sample').last);
    await tester.pumpAndSettle();

    final button = tester.getRect(find.bySemanticsLabel('Sample settings'));
    final theme = MixarThemeData.dark();
    final panel = tester.getRect(find.byType(DeckPadsPanel));
    final select = tester.getRect(find.byType(MixarSelect<PadMode>));
    // The gear owns its whole header cell: inset only by the card frame, and
    // starting exactly where the selector ends.
    final border = theme.style.borderWidth;
    expect(button.right, panel.right - border);
    expect(button.left, select.right);
    expect(button.top, select.top + border);
    expect(button.height, select.height - 2 * border);
    final pads = find.byType(PadButton);
    for (var i = 0; i < pads.evaluate().length; i++) {
      expect(
        tester.getRect(pads.at(i)).top,
        greaterThanOrEqualTo(button.bottom),
      );
    }

    // Anchored to the header, the menu must open into the panel, not above it.
    await tester.tap(find.bySemanticsLabel('Sample settings'));
    await tester.pumpAndSettle();
    expect(
      tester.getRect(find.text('New bank…')).top,
      greaterThanOrEqualTo(button.bottom),
    );
  });

  testWidgets('configurable modes use the same pad size as plain modes', (
    tester,
  ) async {
    // Wide and short: the grid is height-limited, which is where the old
    // floating-button reserve shrank the configurable modes' pads.
    const size = Size(480, 240);

    await pumpPanel(tester, hasTrack: true, size: size);
    final plain = tester.getSize(find.byType(PadButton).first);

    for (final mode in [PadMode.sampler, PadMode.keyboard, PadMode.keyShift]) {
      await pumpPanel(tester, hasTrack: true, size: size, padMode: mode);
      expect(
        tester.getSize(find.byType(PadButton).first),
        plain,
        reason: '$mode pads must match non-configurable modes',
      );
    }
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
    final restingBorder = decoration().border!.top.color;
    expect(
      restingBorder,
      Color.lerp(
        theme.colors.border,
        hotCueAccent(0).fill.withValues(alpha: 1),
        0.4,
      ),
    );
    // The accent fill is a 20% tint; the hairline must still be opaque.
    expect(restingBorder.a, 1.0);

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
