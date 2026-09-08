import 'dart:async';

import 'package:flutter/widgets.dart';
import 'package:forui/forui.dart';

/// Cue amber — same tokens as master PFL (`master_strip.dart` / DESIGN.md).
const _cueOn = Color(0xFFFCD34D); // amber-300
const _cueBorder = Color(0x66F59E0B); // amber-500/40
const _cueFill = Color(0x28F59E0B); // amber-500/16

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
    final enabled = !widget.disabled;
    return Semantics(
      button: true,
      enabled: enabled,
      label: 'Cue',
      child: Listener(
        behavior: HitTestBehavior.opaque,
        onPointerDown: enabled ? (_) => _down() : null,
        onPointerUp: enabled ? (_) => _up() : null,
        onPointerCancel: enabled ? (_) => _up() : null,
        child: ConstrainedBox(
          constraints: const BoxConstraints(minHeight: 36, minWidth: 64),
          child: DecoratedBox(
            decoration: BoxDecoration(
              color: enabled
                  ? _cueFill
                  : theme.colors.secondary.withValues(alpha: 0.35),
              border: Border.all(
                color: enabled ? _cueBorder : theme.colors.border,
              ),
              borderRadius: theme.style.borderRadius.md,
            ),
            child: Center(
              child: Text(
                'Cue',
                style: theme.typography.body.sm.copyWith(
                  color: enabled ? _cueOn : theme.colors.mutedForeground,
                  fontWeight: .w700,
                  letterSpacing: 0.4,
                ),
              ),
            ),
          ),
        ),
      ),
    );
  }
}
