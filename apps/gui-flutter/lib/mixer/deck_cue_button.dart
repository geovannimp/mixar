import 'dart:async';

import 'package:flutter/widgets.dart';
import 'package:forui/forui.dart';

/// Hold to audition cue; tap (under [_holdThreshold]) sets the cue point.
class DeckCueButton extends StatefulWidget {
  const DeckCueButton({
    required this.disabled,
    required this.onBeginHold,
    required this.onEndHold,
    required this.onSetCue,
    super.key,
  });

  final bool disabled;
  final VoidCallback onBeginHold;
  final VoidCallback onEndHold;
  final VoidCallback onSetCue;

  @override
  State<DeckCueButton> createState() => _DeckCueButtonState();
}

class _DeckCueButtonState extends State<DeckCueButton> {
  static const _holdThreshold = Duration(milliseconds: 180);

  Timer? _holdTimer;
  var _auditioning = false;

  void _down() {
    if (widget.disabled || _holdTimer != null || _auditioning) {
      return;
    }
    _holdTimer = Timer(_holdThreshold, () {
      _holdTimer = null;
      _auditioning = true;
      widget.onBeginHold();
    });
  }

  void _up() {
    final timer = _holdTimer;
    _holdTimer = null;
    if (timer != null && timer.isActive) {
      timer.cancel();
      if (!widget.disabled) {
        widget.onSetCue();
      }
      return;
    }
    if (_auditioning) {
      _auditioning = false;
      widget.onEndHold();
    }
  }

  @override
  void didUpdateWidget(covariant DeckCueButton oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (!oldWidget.disabled && widget.disabled) {
      _up();
    }
  }

  @override
  void dispose() {
    _holdTimer?.cancel();
    if (_auditioning) {
      widget.onEndHold();
    }
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    return Semantics(
      button: true,
      enabled: !widget.disabled,
      label: 'Cue',
      child: Listener(
        behavior: HitTestBehavior.opaque,
        onPointerDown: widget.disabled ? null : (_) => _down(),
        onPointerUp: widget.disabled ? null : (_) => _up(),
        onPointerCancel: widget.disabled ? null : (_) => _up(),
        child: ConstrainedBox(
          constraints: const BoxConstraints(minHeight: 36, minWidth: 64),
          child: DecoratedBox(
            decoration: BoxDecoration(
              color: widget.disabled
                  ? theme.colors.secondary.withValues(alpha: 0.35)
                  : theme.colors.secondary.withValues(alpha: 0.45),
              border: Border.all(color: theme.colors.border),
              borderRadius: theme.style.borderRadius.md,
            ),
            child: Center(
              child: Text(
                'Cue',
                style: theme.typography.body.sm.copyWith(
                  color: widget.disabled
                      ? theme.colors.mutedForeground
                      : theme.colors.foreground,
                  fontWeight: .w600,
                ),
              ),
            ),
          ),
        ),
      ),
    );
  }
}
