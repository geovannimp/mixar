import 'package:flutter/widgets.dart';
import 'package:gui_flutter/mixer/pad_modes.dart';
import 'package:gui_flutter/mixer/pads/pad_button.dart';
import 'package:gui_flutter/mixer/pads/pad_grid.dart';
import 'package:gui_flutter/shell/mixar_theme.dart';

/// Keyboard pad mode: eight hold pads playing the scale degrees of [scale].
class KeyboardPads extends StatelessWidget {
  const new({
    required this.scale,
    required this.onPress,
    required this.onRelease,
    this.disabled = false,
    super.key,
  });

  final KeyboardScale scale;
  final ValueChanged<int> onPress;
  final ValueChanged<int> onRelease;
  final bool disabled;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final degrees =
        kKeyboardScaleDegrees[scale] ?? const [0, 2, 4, 5, 7, 9, 11, 12];
    return PadGrid(
      children: [
        for (var slot = 0; slot < 8; slot++)
          () {
            final degree = degrees[slot];
            return HoldPadButton(
              disabled: disabled,
              tooltip: 'Play scale degree ${_formatDegree(degree)} — hold',
              onBegin: () => onPress(slot),
              onEnd: () => onRelease(slot),
              child: Text(
                _formatDegree(degree),
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

String _formatDegree(int degree) => degree > 0 ? '+$degree' : '$degree';
