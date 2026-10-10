import 'dart:async';

import 'package:flutter/widgets.dart';
import 'package:gui_flutter/shell/mixar_theme.dart';
import 'package:shadcn_ui/shadcn_ui.dart';

/// Sides of the [MixarSelect] border stroke.
enum MixarBorderSide { top, right, bottom, left }

/// Mixar select / dropdown over [ShadSelect].
class MixarSelect<T> extends StatefulWidget {
  const new({
    required this.value,
    required this.options,
    required this.labelBuilder,
    required this.onChanged,
    super.key,
    this.subtitleBuilder,
    this.enabled = true,
    this.placeholder,
    this.unfocusAfterPointerSelection = false,
    this.borderRadius,
    this.borderColor,
    this.borderSides,
  });

  final T value;
  final List<T> options;
  final String Function(T value) labelBuilder;
  final String Function(T value)? subtitleBuilder;
  final ValueChanged<T> onChanged;
  final bool enabled;
  final Widget? placeholder;
  final bool unfocusAfterPointerSelection;
  final BorderRadiusGeometry? borderRadius;
  final Color? borderColor;

  /// Sides painted with [borderColor]; `null` paints every side.
  final Set<MixarBorderSide>? borderSides;

  @override
  State<MixarSelect<T>> createState() => _MixarSelectState<T>();
}

class _MixarSelectState<T> extends State<MixarSelect<T>> {
  bool _pointerSelection = false;

  void _resetPointerSelection() {
    scheduleMicrotask(() => _pointerSelection = false);
  }

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;

    Widget buildOption(T option) {
      final child = widget.subtitleBuilder == null
          ? Text(widget.labelBuilder(option))
          : Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              mainAxisSize: MainAxisSize.min,
              children: [
                Text(widget.labelBuilder(option)),
                Text(
                  widget.subtitleBuilder!(option),
                  style: theme.typography.body.xs.copyWith(
                    color: theme.colors.mutedForeground,
                  ),
                ),
              ],
            );
      final selectOption = ShadOption<T>(value: option, child: child);
      if (!widget.unfocusAfterPointerSelection) {
        return selectOption;
      }
      return Listener(
        behavior: HitTestBehavior.translucent,
        onPointerDown: (_) => _pointerSelection = true,
        onPointerUp: (_) => _resetPointerSelection(),
        onPointerCancel: (_) => _resetPointerSelection(),
        child: selectOption,
      );
    }

    return LayoutBuilder(
      builder: (context, constraints) {
        final fill = constraints.maxWidth.isFinite
            ? constraints.maxWidth
            : null;
        return ShadSelect<T>(
          initialValue: widget.value,
          enabled: widget.enabled,
          placeholder: widget.placeholder,
          minWidth: fill,
          decoration: _borderDecoration(theme),
          onChanged: (next) {
            if (next == null) return;
            final pointerSelection = _pointerSelection;
            _pointerSelection = false;
            widget.onChanged(next);
            if (pointerSelection && widget.unfocusAfterPointerSelection) {
              scheduleMicrotask(() {
                if (mounted) {
                  FocusManager.instance.primaryFocus?.unfocus();
                }
              });
            }
          },
          selectedOptionBuilder: (context, selected) =>
              Text(widget.labelBuilder(selected)),
          options: [for (final option in widget.options) buildOption(option)],
        );
      },
    );
  }

  ShadDecoration? _borderDecoration(MixarThemeData theme) {
    final color = widget.borderColor;
    final radius = widget.borderRadius;
    if (color == null && radius == null) return null;
    if (color == null) {
      return ShadDecoration(
        border: ShadBorder(radius: radius),
        secondaryFocusedBorder: ShadBorder(radius: radius),
      );
    }
    final painted = widget.borderSides;
    ShadBorderSide side(MixarBorderSide side) {
      final draw = painted?.contains(side) ?? true;
      return draw
          ? ShadBorderSide(color: color, width: theme.style.borderWidth)
          : ShadBorderSide.none;
    }

    return ShadDecoration(
      border: ShadBorder(
        radius: radius,
        top: side(MixarBorderSide.top),
        right: side(MixarBorderSide.right),
        bottom: side(MixarBorderSide.bottom),
        left: side(MixarBorderSide.left),
      ),
      // Focus outlines the whole control, whatever the base stroke paints.
      secondaryFocusedBorder: ShadBorder(radius: radius),
    );
  }
}
