import 'package:flutter/widgets.dart';
import 'package:gui_flutter/core/utils/performance_modes.dart';
import 'package:gui_flutter/src/rust/api/engine.dart';
import 'package:gui_flutter/widgets/mixer/deck_grid_host.dart';
import 'package:gui_flutter/widgets/mixer/deck_grid_panel.dart';
import 'package:gui_flutter/widgets/mixer/deck_jog_host.dart';
import 'package:gui_flutter/widgets/mixer/deck_loop_host.dart';
import 'package:gui_flutter/widgets/mixer/deck_loop_panel.dart';
import 'package:gui_flutter/widgets/mixer/deck_pads_host.dart';
import 'package:gui_flutter/widgets/mixer/deck_pads_panel.dart';
import 'package:gui_flutter/widgets/mixer/fader_slider.dart';
import 'package:gui_flutter/widgets/mixer/jog_platter.dart';
import 'package:gui_flutter/widgets/shell/app_tooltip.dart';
import 'package:gui_flutter/widgets/shell/m_tabs.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';

/// Pads / Loop / Grid / Jog via vertical [MTabs].
class DeckPerformancePanel extends StatelessWidget {
  const new({
    this.deckId,
    this.hasTrack = false,
    this.disabled = false,
    this.accent = FaderAccent.a,
    super.key,
  });

  final int? deckId;
  final bool hasTrack;

  /// Engine-level disable (not ready / loading). Surfaces that need a loaded
  /// track gate their own controls on [hasTrack]; pad mode stays selectable on
  /// an empty deck.
  final bool disabled;
  final FaderAccent accent;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;

    final radius = theme.style.borderRadius.md;
    final borderW = theme.style.borderWidth;
    // Border is painted in [foregroundDecoration] so the selected tab chip can
    // sit flush against the stroke. An inset [Padding] (MCard's pattern) left a
    // visible gap between the selection wash and the panel hairline.
    return Container(
      decoration: BoxDecoration(
        borderRadius: radius,
        color: theme.colors.background.withValues(alpha: 0.8),
      ),
      foregroundDecoration: BoxDecoration(
        border: Border.all(color: theme.colors.border, width: borderW),
        borderRadius: radius,
      ),
      child: ClipRRect(
        borderRadius: radius,
        child: MTabs(
          direction: Axis.vertical,
          expands: true,
          children: [
            for (final mode in kDeckPerformanceModes)
              MTabEntry(
                label: AppTooltip(
                  tip: deckPerformanceModeLabel(mode),
                  child: Icon(
                    switch (mode) {
                      DeckPerformanceMode.pads => LucideIcons.layoutGrid,
                      DeckPerformanceMode.loop => LucideIcons.repeat2,
                      DeckPerformanceMode.grid => LucideIcons.audioLines,
                      DeckPerformanceMode.jog => LucideIcons.disc3,
                    },
                    size: 16,
                    semanticLabel: deckPerformanceModeLabel(mode),
                  ),
                ),
                child: switch (mode) {
                  DeckPerformanceMode.pads =>
                    deckId != null
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
                            onCreateBank: (_, _) {},
                            onSelectPage: (_) {},
                            onKeyShiftPress: (_) {},
                            onKeyboardPress: (_) {},
                            onKeyboardRelease: (_) {},
                            hasTrack: hasTrack,
                            disabled: disabled,
                            bordered: false,
                          ),
                  DeckPerformanceMode.loop =>
                    deckId != null
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
                  DeckPerformanceMode.grid =>
                    deckId != null
                        ? DeckGridHost(
                            deckId: deckId!,
                            hasTrack: hasTrack,
                            disabled: disabled,
                            bordered: false,
                          )
                        : DeckGridPanel(
                            bpm: null,
                            onSetDownbeat: () {},
                            onNudgeBack: () {},
                            onNudgeForward: () {},
                            onBpmDown: () {},
                            onBpmUp: () {},
                            onBpmSubmit: (_) {},
                            hasTrack: hasTrack,
                            disabled: disabled,
                            bordered: false,
                          ),
                  DeckPerformanceMode.jog =>
                    deckId != null
                        ? Center(
                            child: DeckJogHost(
                              deckId: deckId!,
                              hasTrack: hasTrack,
                              accent: accent,
                              disabled: disabled,
                            ),
                          )
                        : Center(
                            child: JogPlatter(
                              accent: accent,
                              playing: false,
                              hasTrack: hasTrack,
                            ),
                          ),
                },
              ),
          ],
        ),
      ),
    );
  }
}
