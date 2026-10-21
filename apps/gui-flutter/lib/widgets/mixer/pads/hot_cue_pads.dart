import 'package:flutter/widgets.dart';
import 'package:gui_flutter/core/models/pad_modes.dart';
import 'package:gui_flutter/core/utils/pad_format.dart';
import 'package:gui_flutter/widgets/mixer/pads/hold_pad_button.dart';
import 'package:gui_flutter/widgets/mixer/pads/pad_button.dart';
import 'package:gui_flutter/widgets/mixer/pads/pad_face.dart';
import 'package:gui_flutter/widgets/mixer/pads/pad_grid.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';

class DeckHotCue {
  const new({required this.slot, required this.positionMs, this.label});

  final int slot;
  final int positionMs;
  final String? label;
}

class HotCuePads extends StatelessWidget {
  const new({
    required this.hotCues,
    required this.onPress,
    required this.onRelease,
    this.disabled = false,
    super.key,
  });

  final List<DeckHotCue> hotCues;
  final void Function(int slot, bool shift) onPress;
  final ValueChanged<int> onRelease;
  final bool disabled;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    return PadGrid(
      children: [for (var slot = 0; slot < 8; slot++) _pad(theme, slot)],
    );
  }

  Widget _pad(MixarThemeData theme, int slot) {
    DeckHotCue? cue;
    for (final entry in hotCues) {
      if (entry.slot == slot) {
        cue = entry;
        break;
      }
    }
    final filled = cue != null;
    final label = cue?.label?.trim();
    final positionMs = cue?.positionMs;

    final accent = filled ? hotCueAccent(slot) : null;
    return HoldPadButton(
      disabled: disabled,
      fillChild: true,
      chrome: accent == null ? null : accentPadChrome(theme, accent),
      tooltip: filled
          ? 'Pad ${slot + 1} — hold trigger, shift+hold delete'
          : 'Set hot cue on pad ${slot + 1}',
      onBegin: () => onPress(slot, shiftKeyPressed()),
      onEnd: () => onRelease(slot),
      child: PadFace(
        action: 'CUE',
        indicatorColor: accent?.border,
        label: Text(
          filled && label != null && label.isNotEmpty ? label : '${slot + 1}',
          style: theme.typography.body.sm.copyWith(fontWeight: FontWeight.w700),
        ),
        footer: filled && positionMs != null
            ? Text(
                formatDeckTimeTenth(positionMs),
                style: padActionStyle(theme).copyWith(
                  color: theme.colors.mutedForeground,
                  letterSpacing: 0,
                ),
              )
            : null,
      ),
    );
  }
}
