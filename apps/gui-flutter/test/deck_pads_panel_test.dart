import 'package:gui_flutter/shell/material_theme.dart';
import 'package:material_ui/material_ui.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:forui/forui.dart';
import 'package:gui_flutter/mixer/deck_pads_panel.dart';
import 'package:gui_flutter/mixer/pad_modes.dart';
import 'package:gui_flutter/mixer/pads/hot_cue_pads.dart';
import 'package:gui_flutter/mixer/pads/sampler_pads.dart';
import 'package:gui_flutter/settings/settings_defaults.dart';
import 'package:gui_flutter/settings/settings_providers.dart';

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
  }) async {
    var mode = padMode;
    var cues = List<DeckHotCue>.from(hotCues);
    var bankId = activeBankId;
    final theme = FTheme.neutral.dark.desktop;
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          appSettingsProvider.overrideWith((ref) async => defaultAppSettings()),
        ],
        child: MaterialApp(
          theme: materialUiThemeFromForui(theme),
          builder: (context, child) => MaterialUiCompatibilityBridge(
            // ignore: deprecated_member_use
            child: FTheme(data: theme, child: child!),
          ),
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

  testWidgets('mode tabs switch grids', (tester) async {
    await pumpPanel(tester, hasTrack: true);

    expect(find.text('CUE'), findsOneWidget);
    expect(find.text('0:12.5'), findsOneWidget);

    await tester.tap(find.text('JUMP'));
    await tester.pumpAndSettle();
    expect(find.text('+1'), findsOneWidget);
    expect(find.text('0:12.5'), findsNothing);

    await tester.tap(find.text('SAMPLE'));
    await tester.pumpAndSettle();
    expect(find.text('Bank 1'), findsOneWidget);
    expect(find.text('Kick'), findsOneWidget);
  });

  testWidgets('hot cue press on empty slot reports the pad', (tester) async {
    var pressed = <(int, bool)>[];
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
    await tester.tap(find.text('SAMPLE'));
    await tester.pumpAndSettle();

    await tester.tap(find.bySemanticsLabel('Next sampler bank'));
    await tester.pumpAndSettle();
    expect(find.text('Bank 2'), findsOneWidget);
    expect(find.text('hold'), findsOneWidget);
  });

  testWidgets('pad actions stay disabled without a track', (tester) async {
    await pumpPanel(tester, hasTrack: false);

    await tester.tap(find.text('2'));
    await tester.pumpAndSettle();
    expect(find.text('0:01.0'), findsNothing);

    await tester.tap(find.text('ROLL'));
    await tester.pumpAndSettle();
    expect(find.text('roll'), findsWidgets);
  });

  testWidgets('disabled panel blocks mode tabs and pad actions', (
    tester,
  ) async {
    await pumpPanel(tester, hasTrack: true, disabled: true);

    await tester.tap(find.text('JUMP'));
    await tester.pumpAndSettle();
    expect(find.text('+1'), findsNothing);
    expect(find.text('0:12.5'), findsOneWidget);

    await tester.tap(find.text('2'));
    await tester.pumpAndSettle();
    expect(find.text('0:01.0'), findsNothing);
  });
}
