import 'package:flutter/widgets.dart';
import 'package:gui_flutter/mixer/deck_grid_host.dart';
import 'package:gui_flutter/mixer/deck_grid_panel.dart';
import 'package:gui_flutter/mixer/deck_jog.dart';
import 'package:gui_flutter/mixer/deck_loop_host.dart';
import 'package:gui_flutter/mixer/deck_loop_panel.dart';
import 'package:gui_flutter/mixer/deck_pads_host.dart';
import 'package:gui_flutter/mixer/deck_pads_panel.dart';
import 'package:gui_flutter/mixer/fader_slider.dart';
import 'package:gui_flutter/mixer/pad_modes.dart';
import 'package:gui_flutter/mixer/performance_modes.dart';
import 'package:gui_flutter/shell/app_tooltip.dart';
import 'package:gui_flutter/shell/m_tabs.dart';
import 'package:gui_flutter/shell/mixar_theme.dart';
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
  final bool disabled;
  final FaderAccent accent;

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
