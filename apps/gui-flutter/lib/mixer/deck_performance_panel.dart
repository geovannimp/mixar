import 'package:flutter/widgets.dart';
import 'package:forui/forui.dart';
import 'package:gui_flutter/mixer/deck_loop_panel.dart';
import 'package:gui_flutter/mixer/deck_pads_panel.dart';
import 'package:gui_flutter/mixer/performance_modes.dart';

/// [FTabs](https://forui.dev/docs/widgets/navigation/tabs) Pads / Loop content.
///
/// Forui tabs are horizontal-only; [RotatedBox] stands the tab bar on the left
/// and un-rotates each pane.
class DeckPerformancePanel extends StatelessWidget {
  const DeckPerformancePanel({
    this.hasTrack = false,
    this.disabled = false,
    super.key,
  });

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
                    label: Text(deckPerformanceModeLabel(mode)),
                    child: RotatedBox(
                      quarterTurns: 1,
                      child: Directionality(
                        textDirection: TextDirection.ltr,
                        child: switch (mode) {
                          DeckPerformanceMode.pads => DeckPadsPanel(
                            hasTrack: hasTrack,
                            disabled: disabled,
                            bordered: false,
                          ),
                          DeckPerformanceMode.loop => DeckLoopPanel(
                            hasTrack: hasTrack,
                            disabled: disabled,
                            bordered: false,
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
