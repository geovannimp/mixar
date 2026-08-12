import 'package:flutter/widgets.dart';
import 'package:forui/forui.dart';

/// Tauri `AUTO_LOOP_BEATS`.
const kAutoLoopBeats = [1, 2, 4, 8, 16, 32];

/// Index of [beats] in [kAutoLoopBeats], else the index of `4`.
int autoLoopBeatIndex(int beats) {
  final i = kAutoLoopBeats.indexOf(beats);
  return i >= 0 ? i : kAutoLoopBeats.indexOf(4);
}

/// Step [beats] by [delta] slots within [kAutoLoopBeats], clamped.
int stepAutoLoopBeats(int beats, int delta) {
  final next = (autoLoopBeatIndex(beats) + delta).clamp(
    0,
    kAutoLoopBeats.length - 1,
  );
  return kAutoLoopBeats[next];
}

/// Tauri-shaped deck loop controls (local state shell).
///
/// Mounted under the pads panel as a full-width strip.
class DeckLoopPanel extends StatefulWidget {
  const DeckLoopPanel({this.hasTrack = false, this.disabled = false, super.key});

  /// When false, controls are disabled (Tauri `!deck.track`).
  final bool hasTrack;

  final bool disabled;

  @override
  State<DeckLoopPanel> createState() => _DeckLoopPanelState();
}

class _DeckLoopPanelState extends State<DeckLoopPanel> {
  bool _loopActive = false;
  int _loopBeats = 4;

  bool get _controlsDisabled => widget.disabled || !widget.hasTrack;

  void _setLoopLength(int beats) {
    setState(() => _loopBeats = beats);
  }

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final active = _loopActive;
    final disabled = _controlsDisabled;
    final beatIndex = autoLoopBeatIndex(_loopBeats);
    final chipStyle = theme.typography.body.xs.copyWith(
      fontWeight: FontWeight.w700,
      fontSize: 10,
    );
    const compactPad = EdgeInsets.symmetric(horizontal: 4, vertical: 6);
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
    }) {
      return Expanded(
        child: FButton(
          variant: activeVariant(lit),
          size: .sm,
          style: .delta(contentStyle: .delta(padding: .value(compactPad))),
          onPress: (disabled || forceDisabled) ? null : onPress,
          child: Text(label, style: style ?? chipStyle),
        ),
      );
    }

    return DecoratedBox(
      decoration: BoxDecoration(
        border: Border.all(color: borderColor),
        borderRadius: theme.style.borderRadius.md,
        color: fillColor,
      ),
      child: Padding(
        padding: const EdgeInsets.all(6),
        child: Column(
          mainAxisSize: .min,
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
              SizedBox(
                width: double.infinity,
                child: FButton(
                  variant: activeVariant(active),
                  size: .sm,
                  style: .delta(
                    contentStyle: .delta(padding: .value(compactPad)),
                  ),
                  onPress: disabled
                      ? null
                      : () => setState(() => _loopActive = !_loopActive),
                  child: Text('Loop', style: chipStyle),
                ),
              ),
              const SizedBox(height: 4),
              Row(
                children: [
                  cellButton(
                    label: '‹',
                    lit: active,
                    forceDisabled: beatIndex <= 0,
                    onPress: () => _setLoopLength(
                      stepAutoLoopBeats(_loopBeats, -1),
                    ),
                  ),
                  const SizedBox(width: 4),
                  cellButton(
                    label: '$_loopBeats',
                    onPress: null,
                    style: chipStyle.copyWith(
                      fontFeatures: const [FontFeature.tabularFigures()],
                    ),
                  ),
                  const SizedBox(width: 4),
                  cellButton(
                    label: '›',
                    lit: active,
                    forceDisabled: beatIndex >= kAutoLoopBeats.length - 1,
                    onPress: () => _setLoopLength(
                      stepAutoLoopBeats(_loopBeats, 1),
                    ),
                  ),
                ],
              ),
              const SizedBox(height: 4),
              Row(
                children: [
                  cellButton(
                    label: 'IN',
                    lit: active,
                    onPress: () => setState(() => _loopActive = true),
                  ),
                  const SizedBox(width: 4),
                  cellButton(
                    label: 'OUT',
                    lit: active,
                    onPress: () => setState(() => _loopActive = true),
                  ),
                ],
              ),
              const SizedBox(height: 4),
              Row(
                children: [
                  cellButton(label: '-4', onPress: () {}),
                  const SizedBox(width: 4),
                  cellButton(label: '+4', onPress: () {}),
                ],
              ),
            ],
          ),
        ),
      );
  }
}
