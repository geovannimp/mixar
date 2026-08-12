import 'package:flutter/widgets.dart';
import 'package:forui/forui.dart';
import 'package:gui_flutter/mixer/pad_modes.dart';
import 'package:gui_flutter/mixer/pads/pad_button.dart';
import 'package:gui_flutter/mixer/pads/pad_grid.dart';

class BeatJumpPads extends StatelessWidget {
  const BeatJumpPads({
    required this.onBeatJump,
    this.disabled = false,
    super.key,
  });

  final ValueChanged<num> onBeatJump;
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
            return PadButton(
              disabled: disabled,
              tooltip: 'Beat jump ${forward ? '+' : ''}$beats',
              onPress: () => onBeatJump(beats),
              child: Column(
                mainAxisSize: .min,
                children: [
                  Text(
                    forward ? '+$beats' : '$beats',
                    style: theme.typography.body.sm.copyWith(
                      fontWeight: FontWeight.w700,
                    ),
                  ),
                  Text(
                    'beat',
                    style: theme.typography.body.xs.copyWith(
                      fontWeight: FontWeight.w600,
                    ),
                  ),
                ],
              ),
            );
          }(),
      ],
    );
  }
}
