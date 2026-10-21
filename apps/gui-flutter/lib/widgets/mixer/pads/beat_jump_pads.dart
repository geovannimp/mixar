import 'package:flutter/widgets.dart';
import 'package:gui_flutter/core/models/pad_modes.dart';
import 'package:gui_flutter/widgets/mixer/pads/hold_pad_button.dart';
import 'package:gui_flutter/widgets/mixer/pads/pad_face.dart';
import 'package:gui_flutter/widgets/mixer/pads/pad_grid.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';

class BeatJumpPads extends StatelessWidget {
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
            final beats = slot < 4
                ? kBeatJumpForward[slot]
                : kBeatJumpBack[slot - 4];
            final forward = beats > 0;
            return HoldPadButton(
              disabled: disabled,
              fillChild: true,
              tooltip: 'Beat jump ${forward ? '+' : ''}$beats',
              onBegin: () => onPress(slot),
              onEnd: () => onRelease(slot),
              child: PadFace(
                action: 'BEAT',
                label: Text(
                  forward ? '+$beats' : '$beats',
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
