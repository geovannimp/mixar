import 'package:flutter/widgets.dart';
import 'package:gui_flutter/mixer/pad_modes.dart';
import 'package:gui_flutter/mixer/pads/hot_cue_pads.dart' show DeckHotCue;
import 'package:gui_flutter/mixer/pads/pad_button.dart';
import 'package:gui_flutter/mixer/pads/pad_grid.dart';
import 'package:gui_flutter/mixer/pads/pitch_page_bar.dart';
import 'package:gui_flutter/shell/mixar_theme.dart';

/// Keyboard pad mode: eight hold pads playing the semitone [page] relative to
/// the hot-cue root.
///
/// The root is the selected hot cue ([rootHotCue]); a compact chip row lets the
/// user pick it. Empty hot-cue slots are disabled.
class KeyboardPads extends StatelessWidget {
  const new({
    required this.page,
    required this.rootHotCue,
    required this.hotCues,
    required this.onSelectRoot,
    required this.onPress,
    required this.onRelease,
    required this.onPrevPage,
    required this.onNextPage,
    this.disabled = false,
    super.key,
  });

  final int page;
  final int rootHotCue;
  final List<DeckHotCue> hotCues;
  final ValueChanged<int> onSelectRoot;
  final ValueChanged<int> onPress;
  final ValueChanged<int> onRelease;
  final VoidCallback onPrevPage;
  final VoidCallback onNextPage;
  final bool disabled;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final pads = pitchPage(page);
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        _rootSelector(context),
        Expanded(
          child: PadGrid(
            bottomChrome: PitchPageBar(
              page: page,
              onPrev: onPrevPage,
              onNext: onNextPage,
              disabled: disabled,
            ),
            children: [
              for (var slot = 0; slot < 8; slot++)
                () {
                  final pad = pads[slot];
                  return HoldPadButton(
                    disabled: disabled || pad.action == PitchPadAction.none,
                    tooltip: _tooltip(pad),
                    onBegin: () => onPress(slot),
                    onEnd: () => onRelease(slot),
                    child: Text(
                      pitchPadLabel(pad),
                      style: theme.typography.body.sm.copyWith(
                        fontWeight: FontWeight.w700,
                      ),
                    ),
                  );
                }(),
            ],
          ),
        ),
      ],
    );
  }

  Widget _rootSelector(BuildContext context) {
    final filled = {for (final cue in hotCues) cue.slot};
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
      child: Row(
        children: [
          for (var slot = 0; slot < 8; slot++) ...[
            if (slot > 0) const SizedBox(width: 4),
            Expanded(
              child: _RootChip(
                key: ValueKey('keyboard-root-$slot'),
                label: '${slot + 1}',
                selected: slot == rootHotCue,
                enabled: !disabled && filled.contains(slot),
                onTap: () => onSelectRoot(slot),
              ),
            ),
          ],
        ],
      ),
    );
  }
}

class _RootChip extends StatelessWidget {
  const new({
    required this.label,
    required this.selected,
    required this.enabled,
    required this.onTap,
    super.key,
  });

  final String label;
  final bool selected;
  final bool enabled;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final fg = !enabled
        ? theme.colors.mutedForeground.withValues(alpha: 0.45)
        : theme.colors.foreground;
    return GestureDetector(
      behavior: HitTestBehavior.opaque,
      onTap: enabled ? onTap : null,
      child: DecoratedBox(
        decoration: BoxDecoration(
          color: selected
              ? theme.colors.secondary.withValues(alpha: 0.7)
              : const Color(0x00000000),
          border: Border.all(
            color: selected ? theme.colors.foreground : theme.colors.border,
          ),
          borderRadius: theme.style.borderRadius.sm,
        ),
        child: Center(
          child: Text(
            label,
            style: theme.typography.body.xs.copyWith(
              fontWeight: FontWeight.w700,
              color: fg,
            ),
          ),
        ),
      ),
    );
  }
}

String _tooltip(PitchPad pad) => switch (pad.action) {
  PitchPadAction.semitone => 'Play ${pitchPadLabel(pad)} semitones — hold',
  PitchPadAction.keyReset => 'Reset key shift',
  PitchPadAction.semitoneUp => 'Key shift up one semitone',
  PitchPadAction.semitoneDown => 'Key shift down one semitone',
  PitchPadAction.keySync => 'Key sync',
  PitchPadAction.none => '',
};
