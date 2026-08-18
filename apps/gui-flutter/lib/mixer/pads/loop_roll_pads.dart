import 'package:flutter/widgets.dart';
import 'package:forui/forui.dart';
import 'package:gui_flutter/mixer/pad_modes.dart';
import 'package:gui_flutter/mixer/pads/pad_button.dart';
import 'package:gui_flutter/mixer/pads/pad_grid.dart';

class LoopRollPads extends StatelessWidget {
  const LoopRollPads({
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
            return HoldPadButton(
              disabled: disabled,
              tooltip: 'Loop roll $beats beat${beats == 1 ? '' : 's'} — hold',
              onBegin: () => onPress(slot),
              onEnd: () => onRelease(slot),
              child: Column(
                mainAxisSize: .min,
                children: [
                  Text(
                    '$beats',
                    style: theme.typography.body.sm.copyWith(
                      fontWeight: FontWeight.w700,
                    ),
                  ),
                  Text(
                    'roll',
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
