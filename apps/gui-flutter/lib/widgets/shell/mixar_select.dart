import 'dart:async';

import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/providers/settings/settings_providers.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';
import 'package:gui_flutter/widgets/shell/m_tappable.dart';
import 'package:gui_flutter/widgets/shell/mixar_dialog.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:shadcn_ui/shadcn_ui.dart';

/// Sides of the [MixarSelect] border stroke.
enum MixarBorderSide { top, right, bottom, left }

/// Full-width left-aligned option row for the mobile select dialog.
///
/// Avoids [AppButton]'s centered shrink-wrap layout. Selected rows use the
/// Mixar [MixarColors.selection] teal tint (same wash as desktop options).
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
///
/// [options] must be unique under `==` — mobile rows use [ValueKey] of each
/// entry, and duplicate keys confuse Flutter's element reuse.
class MixarSelect<T> extends StatefulWidget {
  const new({
    required this.value,
    required this.options,
    required this.labelBuilder,
    required this.onChanged,
    super.key,
    this.labelSpanBuilder,
    this.optionLabelBuilder,
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

  /// Menu / dialog entries; keep values unique under `==` for stable keys.
  final List<T> options;
  final String Function(T value) labelBuilder;

  /// Optional rich label. Use this when part of the label needs a different
  /// color/weight; [labelBuilder] remains the semantics fallback.
  final InlineSpan? Function(T value)? labelSpanBuilder;

  /// Plain label for dropdown/dialog rows. Defaults to [labelBuilder]; use this
  /// when the collapsed selection carries a suffix the list should omit.
  final String Function(T value)? optionLabelBuilder;
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
  bool _isOpening = false;

  /// Focus of the collapsed control. Owning it lets an outside tap drop the
  /// focus ring, the way a text field's ring goes away when you click away.
  final _focus = FocusNode(debugLabel: 'MixarSelect');

  /// One tap region for the control and its menu, so pressing an option is not
  /// an outside tap.
  final _tapGroup = Object();

  @override
  void dispose() {
    _focus.dispose();
    super.dispose();
  }

  void _resetPointerSelection() {
    scheduleMicrotask(() => _pointerSelection = false);
  }

  Future<void> _openMobilePicker() async {
    if (!widget.enabled || _isOpening) return;
    _isOpening = true;
    try {
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
                const SizedBox(height: 16),
                for (final option in widget.options)
                  Padding(
                    padding: const EdgeInsets.only(bottom: 6),
                    child: _MobileSelectOption(
                      key: ValueKey(option),
                      label: (widget.optionLabelBuilder ?? widget.labelBuilder)(
                        option,
                      ),
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
    } finally {
      _isOpening = false;
    }
  }

  @override
  Widget build(BuildContext context) {
    final override = widget.style;
    final control = override != null
        ? _buildForStyle(effectiveSelectStyle(override))
        : Consumer(
            builder: (context, ref, _) {
              final setting = ref.watch(selectStyleSettingProvider);
              return _buildForStyle(effectiveSelectStyle(setting));
            },
          );
    // Clicking anywhere else ends the interaction and clears the ring. The
    // group covers the menu too, so choosing an option is not "outside".
    return TapRegion(
      groupId: _tapGroup,
      onTapOutside: (_) => _focus.unfocus(),
      child: control,
    );
  }

  Widget _buildForStyle(SelectStyleSetting style) {
    final theme = context.theme;
    final mobile = style == SelectStyleSetting.mobile;

    Widget buildOption(T option) {
      final optionLabel = (widget.optionLabelBuilder ?? widget.labelBuilder)(
        option,
      );
      final label = Text(optionLabel);
      final child = widget.subtitleBuilder == null
          ? label
          : Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              mainAxisSize: MainAxisSize.min,
              children: [
                label,
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
          focusNode: _focus,
          groupId: _tapGroup,
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
          selectedOptionBuilder: (context, selected) {
            final span = widget.labelSpanBuilder?.call(selected);
            return span == null
                ? Text(widget.labelBuilder(selected))
                : Text.rich(span);
          },
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
