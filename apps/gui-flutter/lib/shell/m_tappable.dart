import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';

/// Pointer / focus state for [MTappable] builders.
@immutable
class MTappableState {
  const MTappableState({
    required this.hovered,
    required this.pressed,
    required this.focused,
    required this.disabled,
    required this.selected,
  });

  final bool hovered;
  final bool pressed;
  final bool focused;
  final bool disabled;
  final bool selected;

  /// Forui-equivalent “active” paint (hover, press, or selected).
  bool get active => !disabled && (hovered || pressed || selected);
}

typedef MTappableBuilder =
    Widget Function(BuildContext context, MTappableState state);

/// Lightweight tappable: [MouseRegion] + [GestureDetector], instant state, no bounce.
class MTappable extends StatefulWidget {
  const MTappable({
    required this.builder,
    this.onPress,
    this.onSecondaryPress,
    this.selected = false,
    this.semanticsLabel,
    this.focusNode,
    this.autofocus = false,
    super.key,
  });

  final MTappableBuilder builder;
  final VoidCallback? onPress;
  final VoidCallback? onSecondaryPress;
  final bool selected;
  final String? semanticsLabel;
  final FocusNode? focusNode;
  final bool autofocus;

  @override
  State<MTappable> createState() => _MTappableState();
}

class _MTappableState extends State<MTappable> {
  var _hovered = false;
  var _pressed = false;
  var _focused = false;

  bool get _disabled => widget.onPress == null;

  MTappableState get _state => MTappableState(
    hovered: _hovered,
    pressed: _pressed,
    focused: _focused,
    disabled: _disabled,
    selected: widget.selected,
  );

  void _setHovered(bool value) {
    if (_hovered == value) {
      return;
    }
    setState(() => _hovered = value);
  }

  void _setPressed(bool value) {
    if (_pressed == value) {
      return;
    }
    setState(() => _pressed = value);
  }

  void _setFocused(bool value) {
    if (_focused == value) {
      return;
    }
    setState(() => _focused = value);
  }

  KeyEventResult _onKey(FocusNode node, KeyEvent event) {
    if (_disabled) {
      return KeyEventResult.ignored;
    }
    if (event is! KeyDownEvent) {
      return KeyEventResult.ignored;
    }
    if (event.logicalKey != LogicalKeyboardKey.enter &&
        event.logicalKey != LogicalKeyboardKey.space) {
      return KeyEventResult.ignored;
    }
    widget.onPress?.call();
    return KeyEventResult.handled;
  }

  @override
  Widget build(BuildContext context) {
    final child = widget.builder(context, _state);
    return Semantics(
      button: true,
      enabled: !_disabled,
      label: widget.semanticsLabel,
      child: Focus(
        focusNode: widget.focusNode,
        autofocus: widget.autofocus,
        onFocusChange: _setFocused,
        onKeyEvent: _onKey,
        child: MouseRegion(
          cursor: _disabled
              ? SystemMouseCursors.basic
              : SystemMouseCursors.click,
          onEnter: _disabled ? null : (_) => _setHovered(true),
          onExit: _disabled
              ? null
              : (_) {
                  _setHovered(false);
                  _setPressed(false);
                },
          child: GestureDetector(
            behavior: HitTestBehavior.opaque,
            onTapDown: widget.onPress == null ? null : (_) => _setPressed(true),
            onTapUp: widget.onPress == null ? null : (_) => _setPressed(false),
            onTapCancel: widget.onPress == null
                ? null
                : () => _setPressed(false),
            onTap: widget.onPress,
            onSecondaryTap: widget.onSecondaryPress,
            child: child,
          ),
        ),
      ),
    );
  }
}
