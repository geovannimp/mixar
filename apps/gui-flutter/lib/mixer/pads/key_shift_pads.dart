import 'package:flutter/widgets.dart';
import 'package:gui_flutter/mixer/pad_modes.dart';
import 'package:gui_flutter/mixer/pads/pad_button.dart';
import 'package:gui_flutter/mixer/pads/pad_grid.dart';
import 'package:gui_flutter/mixer/pads/pitch_page_bar.dart';
import 'package:gui_flutter/shell/mixar_theme.dart';

/// Key Shift pad mode: eight latch pads for the semitone [page].
///
/// Press latches the page action (absolute semitones, RESET, UP, DOWN); SYNC is
/// a no-op. This grid only reflects [activeSemitones]; page-5 specials
/// (RESET/UP/DOWN/SYNC) never highlight as an absolute semitone.
class KeyShiftPads extends StatelessWidget {
  const new({
    required this.page,
    required this.activeSemitones,
    required this.onPress,
    required this.onPrevPage,
    required this.onNextPage,
    this.disabled = false,
    super.key,
  });

  final int page;
  final int activeSemitones;
  final ValueChanged<int> onPress;
  final VoidCallback onPrevPage;
  final VoidCallback onNextPage;
  final bool disabled;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final pads = pitchPage(page);
    return PadGrid(
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
            final active =
                pad.action == PitchPadAction.semitone &&
                pad.semitones == activeSemitones;
            return PadButton(
              key: ValueKey('key-shift-pad-$slot${active ? '-active' : ''}'),
              disabled: disabled || pad.action == PitchPadAction.none,
              accentSlot: active ? slot : null,
              tooltip: _tooltip(pad),
              onPress: () => onPress(slot),
              child: Text(
                pitchPadLabel(pad),
                style: theme.typography.body.sm.copyWith(
                  fontWeight: FontWeight.w700,
                ),
              ),
            );
          }(),
      ],
    );
  }
}

String _tooltip(PitchPad pad) => switch (pad.action) {
  PitchPadAction.semitone => 'Key shift ${pitchPadLabel(pad)} semitones',
  PitchPadAction.keyReset => 'Reset key shift',
  PitchPadAction.semitoneUp => 'Key shift up one semitone',
  PitchPadAction.semitoneDown => 'Key shift down one semitone',
  PitchPadAction.keySync => 'Key sync',
  PitchPadAction.none => '',
};
