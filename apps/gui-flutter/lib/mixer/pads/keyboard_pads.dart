import 'package:flutter/widgets.dart';
import 'package:gui_flutter/mixer/mixer_button.dart';
import 'package:gui_flutter/mixer/pad_format.dart';
import 'package:gui_flutter/mixer/pad_modes.dart';
import 'package:gui_flutter/mixer/pads/hot_cue_pads.dart' show DeckHotCue;
import 'package:gui_flutter/mixer/pads/pad_button.dart';
import 'package:gui_flutter/mixer/pads/pad_grid.dart';
import 'package:gui_flutter/mixer/pads/pad_page_pagination.dart';
import 'package:gui_flutter/shell/mixar_dialog.dart';
import 'package:gui_flutter/shell/mixar_theme.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';

/// Keyboard pad mode: eight hold pads playing the semitone [page] relative to
/// the hot-cue root.
///
/// The root is the selected hot cue ([rootHotCue]). The page bar shows the
/// current root and a settings action opens a picker listing only the hot cues
/// that are set.
class KeyboardPads extends StatelessWidget {
  const new({
    required this.page,
    required this.rootHotCue,
    required this.hotCues,
    required this.onSelectRoot,
    required this.onPress,
    required this.onRelease,
    required this.onPrevPage,
    required this.onNextPage,
    this.disabled = false,
    super.key,
  });

  final int page;
  final int rootHotCue;
  final List<DeckHotCue> hotCues;
  final ValueChanged<int> onSelectRoot;
  final ValueChanged<int> onPress;
  final ValueChanged<int> onRelease;
  final VoidCallback onPrevPage;
  final VoidCallback onNextPage;
  final bool disabled;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final pads = keyboardPage(page);
    return PadGrid(
      bottomChrome: pitchPagePagination(
        page: page,
        count: kKeyboardPageCount,
        label: keyboardPageRangeLabel(page),
        onPrevious: onPrevPage,
        onNext: onNextPage,
        centerAccessory: _rootChip(context),
        actionIcon: LucideIcons.settings,
        actionSemanticLabel: 'Keyboard root hot cue',
        onAction: disabled ? null : () => _chooseRoot(context),
        disabled: disabled,
      ),
      children: [
        for (var slot = 0; slot < 8; slot++)
          () {
            final pad = pads[slot];
            return HoldPadButton(
              disabled: disabled || pad.action == PitchPadAction.none,
              tooltip: _tooltip(pad),
              onBegin: () => onPress(slot),
              onEnd: () => onRelease(slot),
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

  Widget _rootChip(BuildContext context) {
    final theme = context.theme;
    final available = hotCues.any((cue) => cue.slot == rootHotCue);
    return Padding(
      padding: const EdgeInsets.only(left: 6),
      child: DecoratedBox(
        decoration: BoxDecoration(
          border: Border.all(color: theme.colors.border),
          borderRadius: BorderRadius.circular(4),
          color: theme.colors.secondary.withValues(alpha: 0.5),
        ),
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: 4, vertical: 1),
          child: Text(
            available ? 'HC ${rootHotCue + 1}' : 'ROOT —',
            style: theme.typography.body.xs.copyWith(
              fontWeight: FontWeight.w700,
            ),
          ),
        ),
      ),
    );
  }

  Future<void> _chooseRoot(BuildContext context) async {
    final available = [...hotCues]..sort((a, b) => a.slot.compareTo(b.slot));
    final selected = await showMixarDialog<int>(
      context: context,
      builder: (context) {
        final theme = context.theme;
        return Padding(
          padding: const EdgeInsets.all(16),
          child: Column(
            mainAxisSize: .min,
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              Text(
                'Root hot cue',
                style: theme.typography.body.md.copyWith(
                  fontWeight: FontWeight.w700,
                ),
              ),
              const SizedBox(height: 4),
              Text(
                available.isEmpty
                    ? 'No hot cues are set for this track.'
                    : 'Keyboard mode plays from the selected hot cue.',
                style: theme.typography.body.xs.copyWith(
                  color: theme.colors.mutedForeground,
                ),
              ),
              const SizedBox(height: 12),
              for (final cue in available) ...[
                _cueOption(context, cue),
                const SizedBox(height: 6),
              ],
              const SizedBox(height: 8),
              MixerButton(
                variant: .secondary,
                onPress: () => Navigator.of(context).pop(),
                child: const Text('Close'),
              ),
            ],
          ),
        );
      },
    );
    if (selected != null) {
      onSelectRoot(selected);
    }
  }

  Widget _cueOption(BuildContext context, DeckHotCue cue) {
    final name = cue.label?.trim();
    final title = name != null && name.isNotEmpty
        ? name
        : 'Hot cue ${cue.slot + 1}';
    return MixerButton(
      variant: cue.slot == rootHotCue ? .primary : .secondary,
      onPress: () => Navigator.of(context).pop(cue.slot),
      child: Text(
        '$title · ${formatDeckTimeTenth(cue.positionMs)}',
        overflow: TextOverflow.ellipsis,
      ),
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
