import 'package:flutter/widgets.dart';
import 'package:gui_flutter/core/models/pad_modes.dart';
import 'package:gui_flutter/widgets/mixer/pads/pad_button.dart';

/// Press-hold pad that calls [onEnd] at most once per [onBegin] (up / cancel / exit).
class HoldPadButton extends StatefulWidget {
  const new({
    required this.child,
    required this.onBegin,
    required this.onEnd,
    this.onPress,
    this.skipBeginWhenShift = false,
    this.disabled = false,
    this.accentSlot,
    this.tooltip,
    super.key,
  });

  final Widget child;
  final VoidCallback onBegin;
  final VoidCallback onEnd;
  final VoidCallback? onPress;
  final bool skipBeginWhenShift;
  final bool disabled;
  final int? accentSlot;
  final String? tooltip;

  @override
  State<HoldPadButton> createState() => _HoldPadButtonState();
}

class _HoldPadButtonState extends State<HoldPadButton> {
  bool _held = false;

  void _begin() {
    if (widget.disabled || _held) {
      return;
    }
    if (widget.skipBeginWhenShift && shiftKeyPressed()) {
      return;
    }
    _held = true;
    widget.onBegin();
  }

  void _end() {
    if (!_held) {
      return;
    }
    _held = false;
    widget.onEnd();
  }

  @override
  void didUpdateWidget(covariant HoldPadButton oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (!oldWidget.disabled && widget.disabled) {
      _end();
    }
  }

  @override
  void dispose() {
    _end();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return PadButton(
      disabled: widget.disabled,
      accentSlot: widget.accentSlot,
      tooltip: widget.tooltip,
      onPress: widget.onPress,
      onPointerDown: _begin,
      onPointerUp: _end,
      onPointerCancel: _end,
      onPointerExit: _end,
      child: widget.child,
    );
  }
}
