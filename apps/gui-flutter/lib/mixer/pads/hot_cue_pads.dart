import 'package:flutter/widgets.dart';
import 'package:forui/forui.dart';
import 'package:gui_flutter/mixer/pad_format.dart';
import 'package:gui_flutter/mixer/pad_modes.dart';
import 'package:gui_flutter/mixer/pads/pad_button.dart';
import 'package:gui_flutter/mixer/pads/pad_grid.dart';

class DeckHotCue {
  const DeckHotCue({
    required this.slot,
    required this.positionMs,
    this.label,
  });

  final int slot;
  final int positionMs;
  final String? label;
}

class HotCuePads extends StatelessWidget {
  const HotCuePads({
    required this.hotCues,
    required this.onTrigger,
    required this.onSave,
    required this.onDelete,
    this.disabled = false,
    super.key,
  });

  final List<DeckHotCue> hotCues;
  final ValueChanged<DeckHotCue> onTrigger;
  final ValueChanged<int> onSave;
  final ValueChanged<int> onDelete;
  final bool disabled;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    return PadGrid(
      children: [
        for (var slot = 0; slot < 8; slot++)
          _pad(theme, slot),
      ],
    );
  }

  Widget _pad(FThemeData theme, int slot) {
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

    return PadButton(
      disabled: disabled,
      accentSlot: filled ? slot : null,
      tooltip: filled
          ? 'Pad ${slot + 1} — click trigger, shift+click delete'
          : 'Set hot cue on pad ${slot + 1}',
      onPress: () {
        if (shiftKeyPressed() && filled) {
          onDelete(slot);
          return;
        }
        final current = cue;
        if (current != null) {
          onTrigger(current);
          return;
        }
        onSave(slot);
      },
      child: Column(
        mainAxisSize: .min,
        children: [
          Text(
            filled && label != null && label.isNotEmpty ? label : '${slot + 1}',
            style: theme.typography.body.sm.copyWith(
              fontWeight: FontWeight.w700,
            ),
          ),
          if (filled && positionMs != null)
            Text(
              formatDeckTimeTenth(positionMs),
              style: theme.typography.body.xs.copyWith(
                fontFeatures: const [FontFeature.tabularFigures()],
              ),
            ),
        ],
      ),
    );
  }
}
