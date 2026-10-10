import 'dart:async';

import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/settings/settings_defaults.dart';
import 'package:gui_flutter/settings/settings_providers.dart';
import 'package:gui_flutter/shell/m_tappable.dart';
import 'package:gui_flutter/shell/mixar_dialog.dart';
import 'package:gui_flutter/shell/mixar_theme.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';
import 'package:shadcn_ui/shadcn_ui.dart';

/// Sides of the [MixarSelect] border stroke.
enum MixarBorderSide { top, right, bottom, left }

/// Full-width left-aligned option row for the mobile select dialog.
///
/// Avoids [AppButton]'s centered shrink-wrap layout. Selected rows use the
/// Mixar [MixarColors.selection] green tint (same wash as desktop options).
class _MobileSelectOption extends StatelessWidget {
  const new({
    required this.label,
    required this.selected,
    required this.onPress,
    this.subtitle,
    super.key,
  });

  final String label;
  final String? subtitle;
  final bool selected;
  final VoidCallback onPress;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    return MTappable(
      onPress: onPress,
      selected: selected,
      semanticsLabel: subtitle == null ? label : '$label. $subtitle',
      builder: (context, state) {
        final fill = selected
            ? theme.colors.selection
            : state.active
            ? theme.colors.secondary
            : theme.colors.card;
        final labelColor = selected
            ? theme.colors.selectionForeground
            : theme.colors.foreground;
        final subtitleColor = selected
            ? theme.colors.selectionForeground.withValues(alpha: 0.75)
            : theme.colors.mutedForeground;
        return DecoratedBox(
          decoration: BoxDecoration(
            color: fill,
            borderRadius: theme.style.borderRadius.md,
            border: Border.all(
              color: theme.colors.border,
              width: theme.style.borderWidth,
            ),
          ),
          child: Padding(
            padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 10),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              mainAxisSize: MainAxisSize.min,
              children: [
                Text(
                  label,
                  textAlign: TextAlign.start,
                  style: theme.typography.body.sm.copyWith(
                    color: labelColor,
                    fontWeight: FontWeight.w600,
                  ),
                ),
                if (subtitle != null) ...[
                  const SizedBox(height: 2),
                  Text(
                    subtitle!,
                    textAlign: TextAlign.start,
                    style: theme.typography.body.xs.copyWith(
                      color: subtitleColor,
                      height: 1.25,
                    ),
                  ),
                ],
              ],
            ),
          ),
        );
      },
    );
  }
}

/// Mixar select / dropdown over [ShadSelect], or a dialog when mobile style.
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
    this.style,
    this.dialogTitle,
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

  /// Force presentation; `null` reads the saved settings style.
  final SelectStyleSetting? style;

  /// Title shown above options in the mobile dialog.
  final String? dialogTitle;

  @override
  State<MixarSelect<T>> createState() => _MixarSelectState<T>();
}

class _MixarSelectState<T> extends State<MixarSelect<T>> {
  bool _pointerSelection = false;

  void _resetPointerSelection() {
    scheduleMicrotask(() => _pointerSelection = false);
  }

  Future<void> _openMobilePicker() async {
    if (!widget.enabled) return;
    final selected = await showMixarDialog<T>(
      context: context,
      builder: (dialogContext) {
        final theme = dialogContext.theme;
        return SingleChildScrollView(
          padding: const EdgeInsets.all(16),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              Text(
                widget.dialogTitle ?? 'Select',
                style: theme.typography.body.md.copyWith(
                  fontWeight: FontWeight.w700,
                ),
              ),
              const SizedBox(height: 12),
              for (final option in widget.options)
                Padding(
                  padding: const EdgeInsets.only(bottom: 6),
                  child: _MobileSelectOption(
                    label: widget.labelBuilder(option),
                    subtitle: widget.subtitleBuilder?.call(option),
                    selected: option == widget.value,
                    onPress: () => Navigator.of(dialogContext).pop(option),
                  ),
                ),
            ],
          ),
        );
      },
    );
    if (selected == null || !mounted) return;
    widget.onChanged(selected);
  }

  @override
  Widget build(BuildContext context) {
    final override = widget.style;
    if (override != null) {
      return _buildForStyle(effectiveSelectStyle(override));
    }
    return Consumer(
      builder: (context, ref, _) {
        final setting = ref.watch(selectStyleSettingProvider);
        return _buildForStyle(effectiveSelectStyle(setting));
      },
    );
  }

  Widget _buildForStyle(SelectStyleSetting style) {
    final theme = context.theme;
    final mobile = style == SelectStyleSetting.mobile;

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
          // Same closed chrome; mobile only replaces the open behavior.
          onPressed: mobile && widget.enabled ? _openMobilePicker : null,
          onChanged: mobile
              ? null
              : (next) {
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
          // Popover options unused when [onPressed] opens the dialog.
          options: mobile
              ? const <Widget>[]
              : [for (final option in widget.options) buildOption(option)],
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
