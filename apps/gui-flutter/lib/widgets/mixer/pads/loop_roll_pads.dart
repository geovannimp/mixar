import 'package:flutter/widgets.dart';
import 'package:gui_flutter/core/models/pad_modes.dart';
import 'package:gui_flutter/core/utils/pad_format.dart';
import 'package:gui_flutter/widgets/mixer/pads/hold_pad_button.dart';
import 'package:gui_flutter/widgets/mixer/pads/pad_face.dart';
import 'package:gui_flutter/widgets/mixer/pads/pad_grid.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';

class LoopRollPads extends StatelessWidget {
  const new({
    required this.onPress,
    required this.onRelease,
    this.disabled = false,
    super.key,
  });

  final ValueChanged<int> onPress;
  final ValueChanged<int> onRelease;
  final bool disabled;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    return PadGrid(
      children: [
        for (var slot = 0; slot < 8; slot++)
          () {
            final beats = kLoopRollBeats[slot];
            final label = formatBeatLength(beats);
            return HoldPadButton(
              disabled: disabled,
              fillChild: true,
              tooltip: 'Loop roll $label beat${beats == 1 ? '' : 's'} — hold',
              onBegin: () => onPress(slot),
              onEnd: () => onRelease(slot),
              child: PadFace(
                action: 'ROLL',
                label: Text(
                  label,
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
