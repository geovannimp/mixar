import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';

/// Pointer / focus state for [MTappable] builders.
@immutable
class MTappableState {
  const new({
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

typedef MTappableBuilder = Widget Function(
  BuildContext context,
  MTappableState state,
);

/// Lightweight tappable: [MouseRegion] + [GestureDetector], instant state, no bounce.
class MTappable extends StatefulWidget {
  const new({
    required this.builder,
    this.onPress,
    this.onSecondaryPress,
    this.selected = false,
    this.semanticsLabel,
    this.focusNode,
    this.autofocus = false,
    super.key,
  });

  /// Static child that ignores hover/press paint (icon chips, dismiss overlays).
  new child({
    required Widget child,
    this.onPress,
    this.onSecondaryPress,
    this.selected = false,
    this.semanticsLabel,
    this.focusNode,
    this.autofocus = false,
    super.key,
  }) : builder = ((_, _) => child);

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
      // Only latched-on controls expose selected; momentary stay unset.
      selected: widget.selected ? true : null,
      label: widget.semanticsLabel,
      child: Focus(
        focusNode: widget.focusNode,
        autofocus: widget.autofocus && !_disabled,
        canRequestFocus: !_disabled,
        skipTraversal: _disabled,
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
            onSecondaryTap: _disabled ? null : widget.onSecondaryPress,
            child: child,
          ),
        ),
      ),
    );
  }
}
