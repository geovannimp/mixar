import 'package:flutter/widgets.dart';
import 'package:forui/forui.dart';
import 'package:gui_flutter/mixer/deck_loop_host.dart';
import 'package:gui_flutter/mixer/deck_loop_panel.dart';
import 'package:gui_flutter/mixer/deck_pads_host.dart';
import 'package:gui_flutter/mixer/deck_pads_panel.dart';
import 'package:gui_flutter/mixer/pad_modes.dart';
import 'package:gui_flutter/mixer/performance_modes.dart';

/// [FTabs](https://forui.dev/docs/widgets/navigation/tabs) Pads / Loop / Jog content.
///
/// Forui tabs are horizontal-only; [RotatedBox] stands the tab bar on the left
/// and un-rotates each pane.
class DeckPerformancePanel extends StatelessWidget {
  const DeckPerformancePanel({
    this.deckId,
    this.hasTrack = false,
    this.disabled = false,
    super.key,
  });

  final int? deckId;
  final bool hasTrack;
  final bool disabled;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;

    return DecoratedBox(
      decoration: BoxDecoration(
        border: Border.all(color: theme.colors.border),
        borderRadius: theme.style.borderRadius.md,
        color: theme.colors.background.withValues(alpha: 0.8),
      ),
      child: ClipRRect(
        borderRadius: theme.style.borderRadius.md,
        child: RotatedBox(
          quarterTurns: 3,
          child: Directionality(
            // RTL puts tab start on the unrotated right → visual top after the 90° turn.
            textDirection: TextDirection.rtl,
            child: FTabs(
              expands: true,
              contentPhysics: const NeverScrollableScrollPhysics(),
              style: .delta(
                spacing: 0,
                indicatorSize: .tab,
                minHeight: 24,
                padding: .value(const .all(4)),
                decoration: DecorationDelta.boxDelta(
                  borderRadius: BorderRadius.zero,
                ),
              ),
              children: [
                for (final mode in kDeckPerformanceModes)
                  FTabEntry(
                    label: RotatedBox(
                      quarterTurns: 1,
                      child: Icon(
                        switch (mode) {
                          DeckPerformanceMode.pads => FLucideIcons.layoutGrid,
                          DeckPerformanceMode.loop => FLucideIcons.repeat2,
                          DeckPerformanceMode.jog => FLucideIcons.disc3,
                        },
                        size: 16,
                        semanticLabel: deckPerformanceModeLabel(mode),
                      ),
                    ),
                    child: RotatedBox(
                      quarterTurns: 1,
                      child: Directionality(
                        textDirection: TextDirection.ltr,
                        child: switch (mode) {
                          DeckPerformanceMode.pads => deckId != null
                              ? DeckPadsHost(
                                  deckId: deckId!,
                                  hasTrack: hasTrack,
                                  disabled: disabled,
                                  bordered: false,
                                )
                              : DeckPadsPanel(
                                  padMode: PadMode.hotCue,
                                  onPadMode: (_) {},
                                  hotCues: const [],
                                  onHotCuePress: (_, _) {},
                                  onHotCueRelease: (_) {},
                                  onLoopRollPress: (_) {},
                                  onLoopRollRelease: (_) {},
                                  onBeatJumpPress: (_) {},
                                  onBeatJumpRelease: (_) {},
                                  samplerSlots: const [],
                                  samplerBanks: const [],
                                  onSamplerPress: (_, _) {},
                                  onSamplerRelease: (_) {},
                                  onSelectBank: (_) {},
                                  onSaveBank: (_, _, _) {},
                                  hasTrack: hasTrack,
                                  disabled: disabled,
                                  bordered: false,
                                ),
                          DeckPerformanceMode.loop => deckId != null
                              ? DeckLoopHost(
                                  deckId: deckId!,
                                  hasTrack: hasTrack,
                                  disabled: disabled,
                                  bordered: false,
                                )
                              : DeckLoopPanel(
                                  loopActive: false,
                                  loopBeats: 4,
                                  onToggleLoop: () {},
                                  onHalveBeats: () {},
                                  onDoubleBeats: () {},
                                  onLoopIn: () {},
                                  onLoopOut: () {},
                                  onBeatsChipPress: () {},
                                  hasTrack: hasTrack,
                                  disabled: disabled,
                                  bordered: false,
                                ),
                          DeckPerformanceMode.jog => const Center(
                            child: _JogPlaceholder(),
                          ),
                        },
                      ),
                    ),
                  ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}

class _JogPlaceholder extends StatelessWidget {
  const _JogPlaceholder();

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    return AspectRatio(
      aspectRatio: 1,
      child: DecoratedBox(
        decoration: BoxDecoration(
          shape: BoxShape.circle,
          border: Border.all(color: theme.colors.border, width: 3),
        ),
        child: Center(
          child: Text(
            'JOG',
            style: theme.typography.body.xs.copyWith(
              color: theme.colors.mutedForeground,
            ),
          ),
        ),
      ),
    );
  }
}
