import 'dart:async';

import 'package:flutter/widgets.dart';
import 'package:gui_flutter/shell/mixar_theme.dart';

/// Hold to audition cue; a short tap sets the cue point.
///
/// Chrome matches deck Play: card + hairline at rest; amber border/label on
/// hover, press, and while auditioning.
class DeckCueButton extends StatefulWidget {
  const new({
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
  var _hovered = false;
  var _pressed = false;

  bool get _accented =>
      !widget.disabled && (_hovered || _pressed || _auditioning);

  void _down() {
    if (widget.disabled || _holdTimer != null || _auditioning) {
      return;
    }
    setState(() => _pressed = true);
    _holdTimer = Timer(_holdThreshold, () {
      _holdTimer = null;
      _auditioning = true;
      widget.onBeginHold();
      if (mounted) {
        setState(() {});
      }
    });
  }

  void _up() {
    final timer = _holdTimer;
    _holdTimer = null;
    if (_pressed) {
      _pressed = false;
      if (mounted) {
        setState(() {});
      }
    }
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
      if (mounted) {
        setState(() {});
      }
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
    final accented = _accented;
    final cueOn = theme.colors.cueOn;
    final borderColor = accented ? cueOn : theme.colors.border;
    final fg = accented ? cueOn : theme.colors.foreground;
    // Soft solid amber only while auditioning — hover stays outline-only.
    final fill = _auditioning ? theme.colors.cueTint : theme.colors.card;

    return Semantics(
      button: true,
      enabled: enabled,
      label: 'Cue',
      child: MouseRegion(
        onEnter: enabled ? (_) => setState(() => _hovered = true) : null,
        onExit: (_) {
          if (_hovered) {
            setState(() => _hovered = false);
          }
        },
        child: Listener(
          behavior: HitTestBehavior.opaque,
          onPointerDown: enabled ? (_) => _down() : null,
          onPointerUp: enabled ? (_) => _up() : null,
          onPointerCancel: enabled ? (_) => _up() : null,
          child: Opacity(
            opacity: enabled ? 1 : 0.5,
            child: ConstrainedBox(
              constraints: const BoxConstraints(minHeight: 36, minWidth: 64),
              child: DecoratedBox(
                decoration: BoxDecoration(
                  color: fill,
                  border: Border.all(
                    color: borderColor,
                    width: theme.style.borderWidth,
                  ),
                  borderRadius: theme.style.borderRadius.md,
                ),
                child: Center(
                  child: Text(
                    'Cue',
                    style: theme.typography.body.sm.copyWith(
                      color: fg,
                      fontWeight: .w700,
                      letterSpacing: 0.4,
                    ),
                  ),
                ),
              ),
            ),
          ),
        ),
      ),
    );
  }
}
