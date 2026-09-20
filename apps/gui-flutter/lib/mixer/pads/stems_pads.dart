import 'package:flutter/widgets.dart';
import 'package:gui_flutter/mixer/pad_modes.dart';
import 'package:gui_flutter/mixer/pads/pad_button.dart';
import 'package:gui_flutter/mixer/pads/pad_grid.dart';
import 'package:gui_flutter/shell/mixar_theme.dart';

/// Stems pad mode: top row mute, bottom row isolate.
class StemsPads extends StatelessWidget {
  const new({
    required this.stemMute,
    required this.stemIsolate,
    required this.stemsReady,
    required this.onPress,
    this.disabled = false,
    super.key,
  });

  final List<bool> stemMute;
  final int? stemIsolate;
  final bool stemsReady;
  final void Function(int slot) onPress;
  final bool disabled;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final inert = disabled || !stemsReady;

    return Column(
      children: [
        if (!stemsReady)
          Padding(
            padding: const EdgeInsets.only(bottom: 4),
            child: Text(
              'STEMS GENERATING…',
              style: theme.typography.body.xs.copyWith(
                color: theme.colors.mutedForeground,
                fontWeight: FontWeight.w600,
                letterSpacing: 1.1,
              ),
            ),
          ),
        Expanded(
          child: PadGrid(
            children: List.generate(8, (slot) {
              final stem = slot % 4;
              final isIsolatePad = slot >= 4;
              final muted = stem < stemMute.length && stemMute[stem];
              final isolated = stemIsolate == stem;
              final lit = stemsReady && (isIsolatePad ? isolated : muted);
              return PadButton(
                disabled: inert,
                accentSlot: lit ? stem : null,
                onPress: inert ? null : () => onPress(slot),
                child: Text(
                  kStemPadLabels[slot],
                  style: theme.typography.body.xs.copyWith(
                    fontWeight: FontWeight.w700,
                    color: inert
                        ? theme.colors.mutedForeground
                        : theme.colors.foreground,
                  ),
                ),
              );
            }),
          ),
        ),
      ],
    );
  }
}
