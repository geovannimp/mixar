import 'package:flutter/widgets.dart';
import 'package:gui_flutter/core/models/pad_modes.dart';
import 'package:gui_flutter/widgets/mixer/pads/hold_pad_button.dart';
import 'package:gui_flutter/widgets/mixer/pads/pad_face.dart';
import 'package:gui_flutter/widgets/mixer/pads/pad_grid.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';

/// Keyboard pad mode: eight hold pads playing the semitone [page] relative to
/// the hot-cue root.
class KeyboardPads extends StatelessWidget {
  const new({
    required this.page,
    required this.onPress,
    required this.onRelease,
    this.disabled = false,
    super.key,
  });

  final int page;
  final ValueChanged<int> onPress;
  final ValueChanged<int> onRelease;
  final bool disabled;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final pads = keyboardPage(page);
    return PadGrid(
      children: [
        for (var slot = 0; slot < 8; slot++)
          () {
            final pad = pads[slot];
            return HoldPadButton(
              disabled: disabled || pad.action == PitchPadAction.none,
              fillChild: true,
              tooltip: _tooltip(pad),
              onBegin: () => onPress(slot),
              onEnd: () => onRelease(slot),
              child: PadFace(
                action: 'NOTE',
                label: Text(
                  pitchPadLabel(pad),
                  style: theme.typography.body.sm.copyWith(
                    fontWeight: FontWeight.w700,
                  ),
                ),
              ),
            );
          }(),
      ],
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
