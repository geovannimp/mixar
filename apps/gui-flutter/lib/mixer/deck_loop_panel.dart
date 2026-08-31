import 'package:flutter/widgets.dart';
import 'package:forui/forui.dart';
import 'package:gui_flutter/shell/app_tooltip.dart';
import 'package:gui_flutter/src/rust/api/library.dart';

/// Tauri `AUTO_LOOP_BEATS`.
const kAutoLoopBeats = [1, 2, 4, 8, 16, 32];

/// Index of [beats] in [kAutoLoopBeats], else the index of `4`.
int autoLoopBeatIndex(int beats) {
  final i = kAutoLoopBeats.indexOf(beats);
  return i >= 0 ? i : kAutoLoopBeats.indexOf(4);
}

/// Step [beats] by [delta] slots within [kAutoLoopBeats], clamped.
int stepAutoLoopBeats(int beats, int delta) {
  final next = (autoLoopBeatIndex(beats) + delta)
      .clamp(0, kAutoLoopBeats.length - 1)
      .toInt();
  return kAutoLoopBeats[next];
}

/// Slot used for save/recall of the loop length chip (Tauri: beat index, max 7).
int autoLoopSlotForBeats(int beats) => autoLoopBeatIndex(beats).clamp(0, 7);

/// Saved loop under [positionMs]: playhead inside `[in, out]`, tightest span,
/// then lowest slot.
SavedLoopInfo? savedLoopAtPosition(
  List<SavedLoopInfo> loops,
  int positionMs,
) {
  SavedLoopInfo? best;
  var bestSpan = 1 << 30;
  for (final loop in loops) {
    if (positionMs < loop.inMs || positionMs > loop.outMs) {
      continue;
    }
    final span = loop.outMs - loop.inMs;
    if (best == null ||
        span < bestSpan ||
        (span == bestSpan && loop.slot < best.slot)) {
      best = loop;
      bestSpan = span;
    }
  }
  return best;
}

/// Nearest listed auto-loop beat length for a region, given track [bpm].
int beatsFromLoopMs({
  required int inMs,
  required int outMs,
  required double? bpm,
}) {
  if (bpm == null || !bpm.isFinite || bpm <= 0) {
    return 4;
  }
  final ms = (outMs - inMs).clamp(1, 1 << 30);
  final beats = ms * bpm / 60000.0;
  var best = kAutoLoopBeats.first;
  var bestDist = (beats - best).abs();
  for (final candidate in kAutoLoopBeats.skip(1)) {
    final dist = (beats - candidate).abs();
    if (dist < bestDist) {
      best = candidate;
      bestDist = dist;
    }
  }
  return best;
}

/// Tauri-shaped deck loop controls.
///
/// ```
/// Loop
/// ‹ beats ›
/// IN  OUT
/// ```
class DeckLoopPanel extends StatelessWidget {
  const DeckLoopPanel({
    required this.loopActive,
    required this.loopBeats,
    required this.onToggleLoop,
    required this.onHalveBeats,
    required this.onDoubleBeats,
    required this.onLoopIn,
    required this.onLoopOut,
    required this.onBeatsChipPress,
    this.savedLoopAtSlot = false,
    this.hasTrack = false,
    this.disabled = false,
    this.bordered = true,
    super.key,
  });

  final bool loopActive;
  final int loopBeats;
  final bool savedLoopAtSlot;
  final VoidCallback onToggleLoop;
  final VoidCallback onHalveBeats;
  final VoidCallback onDoubleBeats;
  final VoidCallback onLoopIn;
  final VoidCallback onLoopOut;
  final VoidCallback onBeatsChipPress;

  /// When false, controls are disabled (Tauri `!deck.track`).
  final bool hasTrack;

  final bool disabled;

  /// When false, skips the outer bordered chrome (parent supplies it).
  final bool bordered;

  bool get _controlsDisabled => disabled || !hasTrack;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final active = loopActive;
    final disabled = _controlsDisabled;
    final beatIndex = autoLoopBeatIndex(loopBeats);
    final chipStyle = theme.typography.body.sm.copyWith(
      fontWeight: FontWeight.w700,
    );
    const buttonPad = EdgeInsets.symmetric(horizontal: 12, vertical: 14);
    final borderColor = active
        ? theme.colors.primary.withValues(alpha: 0.45)
        : theme.colors.border;
    final fillColor = active
        ? theme.colors.primary.withValues(alpha: 0.12)
        : theme.colors.background.withValues(alpha: 0.8);

    FButtonVariant activeVariant(bool lit) => lit ? .secondary : .outline;

    Widget cellButton({
      required String label,
      required VoidCallback? onPress,
      bool lit = false,
      bool forceDisabled = false,
      TextStyle? style,
      String? tip,
    }) {
      final button = FButton(
        variant: activeVariant(lit),
        size: .sm,
        style: .delta(contentStyle: .delta(padding: .value(buttonPad))),
        onPress: (disabled || forceDisabled) ? null : onPress,
        semanticsLabel: tip,
        child: Text(label, style: style ?? chipStyle),
      );
      return Expanded(
        child: tip == null
            ? button
            : AppTooltip(tip: tip, child: button),
      );
    }

    final controls = ConstrainedBox(
      constraints: const BoxConstraints(maxWidth: 280),
      child: Column(
        mainAxisSize: .min,
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          FButton(
            variant: activeVariant(active),
            size: .sm,
            style: .delta(contentStyle: .delta(padding: .value(buttonPad))),
            onPress: disabled ? null : onToggleLoop,
            child: Text('Loop', style: chipStyle),
          ),
          const SizedBox(height: 12),
          Row(
            children: [
              cellButton(
                label: '‹',
                tip: 'Halve loop length',
                lit: active,
                forceDisabled: beatIndex <= 0,
                onPress: onHalveBeats,
              ),
              const SizedBox(width: 8),
              cellButton(
                label: '$loopBeats',
                lit: savedLoopAtSlot,
                onPress: savedLoopAtSlot ? onBeatsChipPress : null,
                style: chipStyle.copyWith(
                  fontFeatures: const [FontFeature.tabularFigures()],
                ),
              ),
              const SizedBox(width: 8),
              cellButton(
                label: '›',
                tip: 'Double loop length',
                lit: active,
                forceDisabled: beatIndex >= kAutoLoopBeats.length - 1,
                onPress: onDoubleBeats,
              ),
            ],
          ),
          const SizedBox(height: 12),
          Row(
            children: [
              cellButton(label: 'IN', lit: active, onPress: onLoopIn),
              const SizedBox(width: 8),
              cellButton(label: 'OUT', lit: active, onPress: onLoopOut),
            ],
          ),
        ],
      ),
    );

    final body = Padding(
      padding: const EdgeInsets.all(16),
      child: Center(child: controls),
    );

    if (!bordered) {
      return ColoredBox(
        color: active ? fillColor : const Color(0x00000000),
        child: body,
      );
    }

    return DecoratedBox(
      decoration: BoxDecoration(
        border: Border.all(color: borderColor),
        borderRadius: theme.style.borderRadius.md,
        color: fillColor,
      ),
      child: body,
    );
  }
}
