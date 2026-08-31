import 'package:flutter/widgets.dart';
import 'package:forui/forui.dart';
import 'package:gui_flutter/mixer/pad_modes.dart';
import 'package:gui_flutter/shell/app_tooltip.dart';

/// Tauri `HOT_CUE_ACCENTS` slot colors (border / fill / text).
({Color border, Color fill, Color text}) hotCueAccent(int slot) {
  const accents = <(int, int, int)>[
    (239, 68, 68), // red-500
    (249, 115, 22), // orange-500
    (234, 179, 8), // yellow-500
    (34, 197, 94), // green-500
    (6, 182, 212), // cyan-500
    (59, 130, 246), // blue-500
    (139, 92, 246), // violet-500
    (236, 72, 153), // pink-500
  ];
  final rgb = accents[((slot % 8) + 8) % 8];
  return (
    border: Color.fromRGBO(rgb.$1, rgb.$2, rgb.$3, 0.55),
    fill: Color.fromRGBO(rgb.$1, rgb.$2, rgb.$3, 0.20),
    text: Color.fromRGBO(
      (rgb.$1 + 255) ~/ 2,
      (rgb.$2 + 255) ~/ 2,
      (rgb.$3 + 255) ~/ 2,
      1,
    ),
  );
}

/// Pad-sized button matching Tauri `DeckButton` size=`pad`.
class PadButton extends StatelessWidget {
  const PadButton({
    required this.child,
    this.onPress,
    this.onPointerDown,
    this.onPointerUp,
    this.onPointerCancel,
    this.onPointerExit,
    this.disabled = false,
    this.accentSlot,
    this.tooltip,
    super.key,
  });

  final Widget child;
  final VoidCallback? onPress;
  final VoidCallback? onPointerDown;
  final VoidCallback? onPointerUp;
  final VoidCallback? onPointerCancel;
  final VoidCallback? onPointerExit;
  final bool disabled;
  final int? accentSlot;
  final String? tooltip;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final accent = accentSlot != null ? hotCueAccent(accentSlot!) : null;
    final border = disabled
        ? theme.colors.border
        : (accent?.border ?? theme.colors.border);
    final fill = disabled
        ? theme.colors.secondary.withValues(alpha: 0.35)
        : (accent?.fill ?? theme.colors.secondary.withValues(alpha: 0.45));
    final fg = disabled
        ? theme.colors.mutedForeground
        : (accent?.text ?? theme.colors.foreground);

    Widget pad = MouseRegion(
      onExit: disabled || onPointerExit == null
          ? null
          : (_) => onPointerExit!(),
      child: Listener(
        behavior: HitTestBehavior.opaque,
        onPointerDown: disabled || onPointerDown == null
            ? null
            : (_) => onPointerDown!(),
        onPointerUp: disabled || onPointerUp == null
            ? null
            : (_) => onPointerUp!(),
        onPointerCancel: disabled || onPointerCancel == null
            ? null
            : (_) => onPointerCancel!(),
        child: GestureDetector(
          behavior: HitTestBehavior.opaque,
          onTap: disabled ? null : onPress,
          child: SizedBox.expand(
            child: DecoratedBox(
              decoration: BoxDecoration(
                color: fill,
                border: Border.all(color: border),
                borderRadius: theme.style.borderRadius.md,
              ),
              child: DefaultTextStyle.merge(
                style: TextStyle(color: fg),
                // ponytail: Inter metrics + two-line cue labels can exceed the
                // square cell by <1px on Flutter 3.47; scale down instead of overflow.
                child: Center(
                  child: FittedBox(fit: .scaleDown, child: child),
                ),
              ),
            ),
          ),
        ),
      ),
    );

    if (tooltip != null && tooltip!.isNotEmpty) {
      pad = AppTooltip(
        tip: tooltip!,
        child: Semantics(label: tooltip, button: true, child: pad),
      );
    }
    return pad;
  }
}

/// Press-hold pad that calls [onEnd] at most once per [onBegin] (up / cancel / exit).
class HoldPadButton extends StatefulWidget {
  const HoldPadButton({
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
