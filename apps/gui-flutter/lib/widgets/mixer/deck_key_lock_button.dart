import 'dart:async';

import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/providers/mixer/engine_providers.dart';
import 'package:gui_flutter/widgets/mixer/mixer_button.dart';
import 'package:gui_flutter/widgets/shell/app_tooltip.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:gui_flutter/widgets/shell/mixar_toast.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';

Future<void> deckSetKeyLock(
  BuildContext context,
  WidgetRef ref,
  int deckId,
  bool enabled,
) async {
  try {
    await setDeckKeyLock(ref, deckId, enabled);
  } catch (e) {
    if (!context.mounted) {
      return;
    }
    showMixarToast(
      context: context,
      variant: MixarToastVariant.destructive,
      title: Text('$e'),
    );
  }
}

/// Ghost track-key control: musical/Camelot label + lock / lock-open icon.
class DeckKeyLockButton extends StatelessWidget {
  const new({
    required this.keyLabel,
    required this.keyLock,
    required this.onChanged,
    this.keyColor,
    this.enabled = true,
    super.key,
  });

  final String keyLabel;
  final Color? keyColor;
  final bool keyLock;
  final bool enabled;

  /// Absolute desired key-lock state from the visible control.
  final ValueChanged<bool> onChanged;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final muted = theme.colors.mutedForeground;
    final tip = keyLock ? 'Key lock on' : 'Key lock off';
    final description = keyLock
        ? 'Keeps musical pitch when you change tempo.'
        : 'Pitch follows tempo changes.';
    return AppTooltip(
      tip: tip,
      description: description,
      child: MixerButton(
        variant: .ghost,
        size: .xs,
        mainAxisSize: .min,
        onPress: enabled
            ? () {
                // Capture absolute intent from chrome; do not re-read later.
                onChanged(!keyLock);
              }
            : null,
        semanticsLabel: tip,
        padding: const EdgeInsets.symmetric(horizontal: 4, vertical: 2),
        child: Row(
          mainAxisSize: .min,
          children: [
            Text(
              keyLabel,
              style: theme.typography.body.xs.copyWith(
                color: keyColor ?? muted,
                fontWeight: .w600,
                fontFeatures: const [FontFeature.tabularFigures()],
              ),
            ),
            const SizedBox(width: 4),
            Icon(
              keyLock ? LucideIcons.lock : LucideIcons.lockOpen,
              size: 12,
              color: muted,
            ),
          ],
        ),
      ),
    );
  }
}
