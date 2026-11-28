import 'package:flutter/gestures.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/models/bus_enums.dart';
import 'package:gui_flutter/core/models/pad_modes.dart';
import 'package:gui_flutter/core/theme/material_theme.dart';
import 'package:gui_flutter/core/theme/mixar_button_style.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/providers/settings/settings_providers.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';
import 'package:gui_flutter/widgets/mixer/deck_pads_panel.dart';
import 'package:gui_flutter/widgets/mixer/mixer_button.dart';
import 'package:gui_flutter/widgets/mixer/pads/hot_cue_pads.dart';
import 'package:gui_flutter/widgets/mixer/pads/key_shift_pads.dart';
import 'package:gui_flutter/widgets/mixer/pads/keyboard_pads.dart';
import 'package:gui_flutter/widgets/mixer/pads/pad_button.dart';
import 'package:gui_flutter/widgets/mixer/pads/pad_grid.dart';
import 'package:gui_flutter/widgets/mixer/pads/sampler_pads.dart';
import 'package:gui_flutter/widgets/shell/mixar_select.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:material_ui/material_ui.dart';
import 'package:shadcn_ui/shadcn_ui.dart';

import 'support/mixar_material_app.dart';
import 'support/pad_finders.dart';
import 'support/render_capture.dart';

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
  final panelKey = GlobalKey();

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
                return RepaintBoundary(
                  key: panelKey,
                  child: SizedBox(
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

  /// Geometry the header shares at every width: the select starts `inset`
  /// from the panel, the gear ends `inset` from it, and `gap` separates them.
  void expectHeaderLayout(
    WidgetTester tester, {
    required Rect panel,
    required Rect select,
    required Rect gear,
    double inset = 8,
    double gap = 8,
    Size gearSize = const Size(32, 32),
  }) {
    expect(gear.size, gearSize);
    expect(select.left - panel.left, inset);
    expect(panel.right - gear.right, inset);
    expect(gear.left - select.right, gap);
    expect(gear.center.dy, moreOrLessEquals(select.center.dy, epsilon: 0.5));
  }

  Rect panelRect(WidgetTester tester) =>
      tester.getRect(find.byType(DeckPadsPanel));

  Rect selectRect(WidgetTester tester) =>
      tester.getRect(find.byType(MixarSelect<PadMode>));

  Rect gearRect(WidgetTester tester) => tester.getRect(
    find.descendant(
      of: find.byType(DeckPadsPanel),
      matching: find.byType(MixerButton),
    ),
  );

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

  testWidgets('the mode select keeps the shared rounded border', (
    tester,
  ) async {
    await pumpPanel(tester, hasTrack: true);
    final select = tester.widget<MixarSelect<PadMode>>(
      find.byType(MixarSelect<PadMode>),
    );
    // No per-header overrides: the shared select draws the full control.
    expect(select.borderRadius, isNull);
    expect(select.borderColor, isNull);
    expect(select.borderSides, isNull);

    final theme = MixarThemeData.dark();
    final frame = tester
        .widgetList<DecoratedBox>(
          find.ancestor(
            of: find.byType(MixarSelect<PadMode>),
            matching: find.byType(DecoratedBox),
          ),
        )
        .map((widget) => widget.decoration)
        .whereType<BoxDecoration>()
        .firstWhere(
          (decoration) =>
              decoration.borderRadius != null && decoration.border != null,
        );
    final frameBorder = frame.border! as Border;
    expect(frameBorder.top.color, theme.colors.border);
    expect(frameBorder.right.color, theme.colors.border);
    expect(frameBorder.bottom.color, theme.colors.border);
  });

  testWidgets("the pad table's top edge sits under the mode header", (
    tester,
  ) async {
    await pumpPanel(tester, hasTrack: true);
    final header = tester.getRect(find.byType(MixarSelect<PadMode>));
    final firstPad = tester.getRect(find.byType(PadButton).first);
    final lastPad = tester.getRect(find.byType(PadButton).at(3));

    // The grid paints that edge itself, snapped to the screen's pixels, so a
    // header rule beside it would double the line.
    final seam = find.byWidgetPredicate(
      (widget) => widget is CustomPaint && widget.painter is PadTablePainter,
      description: 'table seams',
    );
    final layer = tester.getRect(seam);
    expect(layer.top, closeTo(firstPad.top, 0.5));
    expect(layer.top, greaterThanOrEqualTo(header.bottom));
    expect(layer.left, closeTo(firstPad.left, 0.5));
    expect(layer.right, closeTo(lastPad.right, 0.5));

    // Painted, not just computed: the grid's first pixel row is the seam.
    final bytes = await captureRgba(tester, panelKey);
    final imageWidth = tester.getRect(find.byKey(panelKey)).width.round();
    final i = (layer.top.round() * imageWidth + firstPad.center.dx.round()) * 4;
    final border = MixarThemeData.dark().colors.border;
    expect(
      (bytes[i] << 16) | (bytes[i + 1] << 8) | bytes[i + 2],
      (border.r * 255).round() << 16 |
          (border.g * 255).round() << 8 |
          (border.b * 255).round(),
    );
  });

  testWidgets('the grid paints three column seams and one row seam', (
    tester,
  ) async {
    await pumpPanel(tester, hasTrack: true);
    final line = tester.getRect(
      find.byWidgetPredicate(
        (widget) => widget is CustomPaint && widget.painter is PadTablePainter,
        description: 'table seams',
      ),
    );
    final firstPad = tester.getRect(find.byType(PadButton).first);
    final lastPad = tester.getRect(find.byType(PadButton).at(7));

    // The seam layer covers exactly the tiled block.
    expect(line.left, closeTo(firstPad.left, 0.5));
    expect(line.top, closeTo(firstPad.top, 0.5));
    expect(line.right, closeTo(lastPad.right, 0.5));
    expect(line.bottom, closeTo(lastPad.bottom, 0.5));
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
    expect(button.variant, MixarButtonVariant.outline);
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

  testWidgets('wheel over the bank list scrolls the menu', (tester) async {
    tester.view.physicalSize = const Size(800, 180);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);

    await pumpPanel(
      tester,
      hasTrack: true,
      padMode: PadMode.sampler,
      size: const Size(360, 260),
      samplerBanks: [
        for (var i = 0; i < 20; i++)
          SamplerBank(id: 'bank-$i', name: 'Bank $i'),
      ],
    );
    await tester.tap(find.bySemanticsLabel('Sample settings'));
    await tester.pumpAndSettle();

    // The bank rows have no scroll view of their own; the wheel must move the
    // single menu viewport. Twenty banks overflow the old 240 px inner cap.
    final ancestorScrollables = find.ancestor(
      of: find.text('Bank 1'),
      matching: find.byType(Scrollable),
    );
    expect(ancestorScrollables, findsOneWidget);
    final outer = tester.state<ScrollableState>(ancestorScrollables).position;
    expect(outer.maxScrollExtent, greaterThan(0));
    expect(outer.pixels, 0);

    await tester.sendEventToBinding(
      PointerScrollEvent(
        position: tester.getCenter(find.text('Bank 1')),
        scrollDelta: const Offset(0, 120),
      ),
    );
    await tester.pumpAndSettle();

    expect(outer.pixels, greaterThan(0));
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
    // The fill carries the state; the grid owns the seam.
    expect(padDecorationOf(tester, pad).color, theme.colors.primary);

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

  testWidgets('a narrow configurable header keeps both controls', (
    tester,
  ) async {
    await pumpPanel(
      tester,
      hasTrack: true,
      size: const Size(81, 280),
      padMode: PadMode.sampler,
    );
    expect(tester.takeException(), isNull);

    // The narrow header drops the insets and shrinks the gear so the select
    // keeps room for its label row and chevron.
    expectHeaderLayout(
      tester,
      panel: panelRect(tester),
      select: selectRect(tester),
      gear: gearRect(tester),
      inset: 0,
      gap: 4,
      gearSize: const Size(24, 24),
    );
    expect(
      tester.getRect(find.byType(MixarSelect<PadMode>)).width,
      greaterThan(40),
    );
  });

  testWidgets('the header narrows in steps and drops the gear at its floor', (
    tester,
  ) async {
    // 112 px: the wide tier keeps the eight-pixel insets and 32 px gear.
    await pumpPanel(
      tester,
      hasTrack: true,
      size: const Size(112, 280),
      padMode: PadMode.sampler,
    );
    expectHeaderLayout(
      tester,
      panel: panelRect(tester),
      select: selectRect(tester),
      gear: gearRect(tester),
    );

    // 111 px: the narrow tier trades the insets and gear size for the select.
    await pumpPanel(
      tester,
      hasTrack: true,
      size: const Size(111, 280),
      padMode: PadMode.sampler,
    );
    expectHeaderLayout(
      tester,
      panel: panelRect(tester),
      select: selectRect(tester),
      gear: gearRect(tester),
      inset: 0,
      gap: 4,
      gearSize: const Size(24, 24),
    );

    // 72 px: the narrowest header that still holds the select and the gear.
    await pumpPanel(
      tester,
      hasTrack: true,
      size: const Size(72, 280),
      padMode: PadMode.sampler,
    );
    expect(tester.takeException(), isNull);
    expectHeaderLayout(
      tester,
      panel: panelRect(tester),
      select: selectRect(tester),
      gear: gearRect(tester),
      inset: 0,
      gap: 4,
      gearSize: const Size(24, 24),
    );

    // 71 px: the gear yields so the select keeps the full width instead of
    // overflowing its label row.
    await pumpPanel(
      tester,
      hasTrack: true,
      size: const Size(71, 280),
      padMode: PadMode.sampler,
    );
    expect(tester.takeException(), isNull);
    expect(find.byType(MixerButton), findsNothing);
    expect(tester.getRect(find.byType(MixarSelect<PadMode>)).width, 71);
  });

  testWidgets('config button sits after the select in the header', (
    tester,
  ) async {
    await pumpPanel(tester, hasTrack: true);
    await tester.tap(find.byType(MixarSelect<PadMode>));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Sample').last);
    await tester.pumpAndSettle();

    final gearFinder = find.descendant(
      of: find.byType(DeckPadsPanel),
      matching: find.byType(MixerButton),
    );
    final gear = tester.getRect(gearFinder);
    expectHeaderLayout(
      tester,
      panel: panelRect(tester),
      select: selectRect(tester),
      gear: gear,
    );
    expect(
      tester.widget<MixerButton>(gearFinder).variant,
      MixarButtonVariant.outline,
    );

    final pads = find.byType(PadButton);
    for (var i = 0; i < pads.evaluate().length; i++) {
      expect(tester.getRect(pads.at(i)).top, greaterThanOrEqualTo(gear.bottom));
    }

    // Anchored to the header, the menu must open into the panel, not above it.
    await tester.tap(find.bySemanticsLabel('Sample settings'));
    await tester.pumpAndSettle();
    expect(
      tester.getRect(find.text('New bank…')).top,
      greaterThanOrEqualTo(gear.bottom),
    );
  });

  testWidgets('config button clears the select in a tabs-hosted panel', (
    tester,
  ) async {
    await pumpPanel(
      tester,
      hasTrack: true,
      bordered: false,
      padMode: PadMode.sampler,
    );
    expectHeaderLayout(
      tester,
      panel: panelRect(tester),
      select: selectRect(tester),
      gear: gearRect(tester),
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

  testWidgets('assigned cue rests neutral and fills while held', (
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
    // Pads are fill only: the accent dot and the held fill carry the state.
    expect(decoration().border, isNull);

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
