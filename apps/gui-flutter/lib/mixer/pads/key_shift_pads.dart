import 'package:flutter/widgets.dart';
import 'package:gui_flutter/mixer/pad_modes.dart';
import 'package:gui_flutter/mixer/pads/pad_button.dart';
import 'package:gui_flutter/mixer/pads/pad_grid.dart';
import 'package:gui_flutter/shell/mixar_theme.dart';

/// Key Shift pad mode: eight latch pads selecting a semitone offset.
///
/// The engine owns the latch (re-pressing the active pad clears it back to
/// `0`); this grid only reflects [activeSemitones].
class KeyShiftPads extends StatelessWidget {
  const new({
    required this.activeSemitones,
    required this.onPress,
    this.disabled = false,
    super.key,
  });

  final int activeSemitones;
  final ValueChanged<int> onPress;
  final bool disabled;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    return PadGrid(
      children: [
        for (var slot = 0; slot < 8; slot++)
          () {
            final semitones = kKeyShiftPadSemitones[slot];
            final active = semitones == activeSemitones;
            return PadButton(
              key: ValueKey('key-shift-pad-$slot${active ? '-active' : ''}'),
              disabled: disabled,
              accentSlot: active ? slot : null,
              tooltip: 'Key shift ${_formatSemitones(semitones)} semitones',
              onPress: () => onPress(slot),
              child: Text(
                _formatSemitones(semitones),
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

String _formatSemitones(int semitones) =>
    semitones > 0 ? '+$semitones' : '$semitones';
