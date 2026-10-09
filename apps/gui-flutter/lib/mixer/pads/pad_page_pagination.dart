import 'package:flutter/widgets.dart';
import 'package:gui_flutter/mixer/mixer_button.dart';
import 'package:gui_flutter/mixer/pad_modes.dart';
import 'package:gui_flutter/shell/mixar_theme.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';

/// Shared bottom-chrome pagination bar for the pad modes.
///
/// Renders `‹  <label>  ›` with an optional [centerAccessory] beside the label
/// (e.g. the sampler play-mode chip) and an optional trailing action button
/// (e.g. the sampler bank-settings gear). Used by Keyboard, Key Shift and
/// Sampler so the page/bank chrome is one control family.
class PadPagePagination extends StatelessWidget {
  const new({
    required this.count,
    required this.label,
    required this.onPrevious,
    required this.onNext,
    this.previousSemanticLabel = 'Previous page',
    this.nextSemanticLabel = 'Next page',
    this.centerAccessory,
    this.actionIcon,
    this.actionSemanticLabel,
    this.onAction,
    this.disabled = false,
    super.key,
  });

  /// Total number of pages.
  final int count;

  /// Center label (page name / bank name).
  final String label;

  final VoidCallback onPrevious;
  final VoidCallback onNext;

  final String previousSemanticLabel;
  final String nextSemanticLabel;

  /// Optional widget shown to the right of [label] (e.g. a mode chip).
  final Widget? centerAccessory;

  /// Optional trailing action button.
  final IconData? actionIcon;
  final String? actionSemanticLabel;
  final VoidCallback? onAction;

  final bool disabled;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final paginationDisabled = disabled || count < 2;
    final actionEnabled = !disabled && onAction != null;
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 2, vertical: 2),
      child: Row(
        children: [
          _PadChromeButton(
            icon: LucideIcons.chevronLeft,
            semanticLabel: previousSemanticLabel,
            disabled: paginationDisabled,
            onPress: onPrevious,
          ),
          Expanded(
            child: Row(
              children: [
                const Spacer(),
                Flexible(
                  child: Text(
                    label,
                    overflow: TextOverflow.ellipsis,
                    textAlign: TextAlign.center,
                    style: theme.typography.body.xs.copyWith(
                      fontWeight: FontWeight.w600,
                    ),
                  ),
                ),
                Expanded(
                  child: Align(
                    alignment: Alignment.centerLeft,
                    child: centerAccessory ?? const SizedBox.shrink(),
                  ),
                ),
              ],
            ),
          ),
          _PadChromeButton(
            icon: LucideIcons.chevronRight,
            semanticLabel: nextSemanticLabel,
            disabled: paginationDisabled,
            onPress: onNext,
          ),
          if (actionIcon != null)
            _PadChromeButton(
              icon: actionIcon!,
              semanticLabel: actionSemanticLabel ?? 'Action',
              disabled: !actionEnabled,
              onPress: onAction,
            ),
        ],
      ),
    );
  }
}

/// Semitone page selector for the Keyboard / Key Shift modes.
///
/// Thin wrapper over [PadPagePagination] with the shared page label and
/// semantic labels.
PadPagePagination pitchPagePagination({
  required int page,
  required VoidCallback onPrevious,
  required VoidCallback onNext,
  Widget? centerAccessory,
  IconData? actionIcon,
  String? actionSemanticLabel,
  VoidCallback? onAction,
  bool disabled = false,
}) {
  return PadPagePagination(
    count: kPitchPageCount,
    label: 'PAGE $page/$kPitchPageCount',
    previousSemanticLabel: 'Previous semitone page',
    nextSemanticLabel: 'Next semitone page',
    onPrevious: onPrevious,
    onNext: onNext,
    centerAccessory: centerAccessory,
    actionIcon: actionIcon,
    actionSemanticLabel: actionSemanticLabel,
    onAction: onAction,
    disabled: disabled,
  );
}

class _PadChromeButton extends StatelessWidget {
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
