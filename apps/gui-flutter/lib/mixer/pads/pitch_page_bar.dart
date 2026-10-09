import 'package:flutter/widgets.dart';
import 'package:gui_flutter/mixer/mixer_button.dart';
import 'package:gui_flutter/mixer/pad_modes.dart';
import 'package:gui_flutter/shell/mixar_theme.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';

/// Semitone page selector chrome (`chevron  PAGE n/5  chevron`) shared by the
/// Keyboard and Key Shift pad modes, mirroring the sampler bank bar.
///
/// Sits inside `PadGrid.bottomChrome`; both buttons disable when [disabled] or
/// when there is only one page ([kPitchPageCount] < 2).
class PitchPageBar extends StatelessWidget {
  const new({
    required this.page,
    required this.onPrev,
    required this.onNext,
    this.disabled = false,
    super.key,
  });

  /// Current page (`1..kPitchPageCount`).
  final int page;
  final VoidCallback onPrev;
  final VoidCallback onNext;
  final bool disabled;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final buttonsDisabled = disabled || kPitchPageCount < 2;
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 2, vertical: 2),
      child: Row(
        children: [
          _PageChromeButton(
            icon: LucideIcons.chevronLeft,
            semanticLabel: 'Previous semitone page',
            disabled: buttonsDisabled,
            onPress: onPrev,
          ),
          Expanded(
            child: Text(
              'PAGE $page/$kPitchPageCount',
              textAlign: TextAlign.center,
              style: theme.typography.body.xs.copyWith(
                fontWeight: FontWeight.w600,
              ),
            ),
          ),
          _PageChromeButton(
            icon: LucideIcons.chevronRight,
            semanticLabel: 'Next semitone page',
            disabled: buttonsDisabled,
            onPress: onNext,
          ),
        ],
      ),
    );
  }
}

class _PageChromeButton extends StatelessWidget {
  const new({
    required this.icon,
    required this.semanticLabel,
    required this.onPress,
    this.disabled = false,
  });

  final IconData icon;
  final String semanticLabel;
  final VoidCallback? onPress;
  final bool disabled;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    return Semantics(
      button: true,
      label: semanticLabel,
      enabled: !disabled,
      child: MixerButton(
        variant: .ghost,
        size: .xs,
        mainAxisSize: .min,
        onPress: disabled ? null : onPress,
        padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 4),
        child: ExcludeSemantics(
          child: Icon(
            icon,
            size: 14,
            color: disabled
                ? theme.colors.mutedForeground.withValues(alpha: 0.4)
                : theme.colors.mutedForeground,
          ),
        ),
      ),
    );
  }
}
