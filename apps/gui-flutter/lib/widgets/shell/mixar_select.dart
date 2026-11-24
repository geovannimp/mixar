import 'dart:async';

import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/core/theme/shad_theme.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/providers/settings/settings_providers.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';
import 'package:gui_flutter/widgets/shell/m_divider.dart';
import 'package:gui_flutter/widgets/shell/m_tappable.dart';
import 'package:gui_flutter/widgets/shell/mixar_dialog.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:shadcn_ui/shadcn_ui.dart';

/// Sides of the [MixarSelect] border stroke.
enum MixarBorderSide { top, right, bottom, left }

/// Full-width left-aligned option row for the mobile select dialog.
///
/// Avoids `AppButton`'s centered shrink-wrap layout. Rows are a flat list
/// separated by hairlines — no per-row frame — and the selected row uses the
/// Mixar `MixarColors.selection` teal tint (same wash as desktop options).
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
          decoration: BoxDecoration(color: fill),
          child: Padding(
            padding: const EdgeInsets.symmetric(horizontal: 24, vertical: 12),
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

  /// Tracks the desktop dropdown so the trigger can join the open panel into
  /// one card instead of keeping its closed pill.
  final ShadPopoverController _popoverController = ShadPopoverController();

  /// Laid-out trigger height, used for the open card's cap.
  final GlobalKey _triggerKey = GlobalKey();
  double? _triggerHeight;

  /// First menu row, used to tell whether the fallback opened the panel above
  /// the trigger (near the bottom of the window).
  final GlobalKey _firstOptionKey = GlobalKey();

  /// `null` until the first open frame is measured; then the join direction.
  bool? _openAbove;

  @override
  void initState() {
    super.initState();
    _popoverController.addListener(_onPopoverChanged);
  }

  void _onPopoverChanged() {
    // Keep the last direction while the panel animates out, but start each
    // open from the closed pill until the new direction is measured.
    if (!_popoverController.isOpen) return;
    if (_openAbove != null) setState(() => _openAbove = null);
    // The panel is laid out in the overlay after this frame; measure then.
    WidgetsBinding.instance.addPostFrameCallback((_) => _detectOpenDirection());
  }

  void _detectOpenDirection() {
    if (!mounted || !_popoverController.isOpen) return;
    final trigger = _triggerKey.currentContext?.findRenderObject();
    final panel = _panelRenderObject(_firstOptionKey.currentContext);
    if (trigger is! RenderBox ||
        panel is! RenderBox ||
        !trigger.hasSize ||
        !panel.hasSize) {
      // The overlay may not be laid out on the first frame; try again.
      WidgetsBinding.instance.addPostFrameCallback((_) {
        _detectOpenDirection();
      });
      return;
    }
    // The panel's own top, not the first row's, stays reliable when a long
    // list scrolls the selected option into view.
    final triggerTop = trigger.localToGlobal(Offset.zero).dy;
    final panelTop = panel.localToGlobal(Offset.zero).dy;
    final above = panelTop < triggerTop;
    final height = trigger.size.height;
    if (above != _openAbove || height != _triggerHeight) {
      setState(() {
        _openAbove = above;
        _triggerHeight = height;
      });
    }
  }

  /// Nearest [ShadDecorator] ancestor of the menu's first option, i.e. the
  /// open panel itself.
  RenderObject? _panelRenderObject(BuildContext? context) {
    if (context == null) return null;
    RenderObject? panel;
    context.visitAncestorElements((element) {
      if (element.widget is ShadDecorator) {
        panel = element.findRenderObject();
        return false;
      }
      return true;
    });
    return panel;
  }

  @override
  void dispose() {
    _popoverController.removeListener(_onPopoverChanged);
    _popoverController.dispose();
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
          return MixarDialog(
            title: widget.dialogTitle ?? 'Select',
            // Rows are full-bleed bands: they paint and separate edge to edge,
            // with the text inset by their own horizontal padding.
            contentPadding: EdgeInsets.zero,
            child: Column(
              mainAxisSize: MainAxisSize.min,
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                for (final (index, option) in widget.options.indexed) ...[
                  if (index > 0)
                    MDivider(
                      padding: EdgeInsets.zero,
                      color: context.theme.colors.border,
                    ),
                  _MobileSelectOption(
                    key: ValueKey(option),
                    label: (widget.optionLabelBuilder ?? widget.labelBuilder)(
                      option,
                    ),
                    subtitle: widget.subtitleBuilder?.call(option),
                    selected: option == widget.value,
                    onPress: () => Navigator.of(dialogContext).pop(option),
                  ),
                ],
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
        return ListenableBuilder(
          listenable: _popoverController,
          builder: (context, _) {
            final open = _popoverController.isOpen;
            final above = _openAbove ?? false;
            // Wait one frame for the measured direction before reshaping the
            // trigger; until then it keeps its closed pill so nothing flashes.
            final joined = open && _openAbove != null;
            final cap = _triggerHeight == null
                ? theme.style.borderRadius.dialog.topLeft
                : Radius.circular(_triggerHeight! / 2);
            // The open card squares off at the join: trigger above the menu
            // keeps its top cap, trigger below it keeps its bottom cap.
            final radius =
                widget.borderRadius ??
                (joined
                    ? (above
                          ? BorderRadius.vertical(bottom: cap)
                          : BorderRadius.vertical(top: cap))
                    : null);
            // The join padding insets the trigger's content by 1px on each
            // side; take it back out of the select's own padding so the label
            // and chevron do not move when the menu opens.
            EdgeInsets? openSelectPadding;
            if (joined) {
              final base =
                  ShadTheme.of(context).selectTheme.padding ??
                  const EdgeInsets.symmetric(horizontal: 12, vertical: 8);
              final resolved = base.resolve(Directionality.of(context));
              openSelectPadding = resolved.copyWith(
                left: resolved.left - theme.style.borderWidth,
                right: resolved.right - theme.style.borderWidth,
              );
            }
            // The popover theme is global; nest one so this select can round
            // the menu's outer end on whichever side it actually opened.
            final baseShad = ShadTheme.of(context);
            final shadTheme = baseShad.copyWith(
              popoverTheme: baseShad.popoverTheme.copyWith(
                decoration: _popoverDecoration(theme, baseShad, above: above),
              ),
              selectTheme: baseShad.selectTheme.copyWith(
                effects: above
                    ? mixarSelectOpenEffects(alignment: Alignment.bottomCenter)
                    : baseShad.selectTheme.effects,
              ),
            );
            return ShadTheme(
              data: shadTheme,
              child: SizedBox(
                key: _triggerKey,
                child: ShadSelect<T>(
                  initialValue: widget.value,
                  enabled: widget.enabled,
                  placeholder: widget.placeholder,
                  minWidth: fill,
                  padding: openSelectPadding,
                  popoverController: _popoverController,
                  decoration: _borderDecoration(
                    theme,
                    radius,
                    fill: joined ? theme.colors.card : null,
                    join: joined,
                  ),
                  // Same closed chrome; mobile only replaces the open behavior.
                  onPressed: mobile && widget.enabled
                      ? _openMobilePicker
                      : null,
                  onChanged: mobile
                      ? null
                      : (next) {
                          if (next == null) return;
                          final pointerSelection = _pointerSelection;
                          _pointerSelection = false;
                          widget.onChanged(next);
                          if (pointerSelection &&
                              widget.unfocusAfterPointerSelection) {
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
                      : [
                          for (final (index, option) in widget.options.indexed)
                            if (index == 0)
                              SizedBox(
                                key: _firstOptionKey,
                                child: buildOption(option),
                              )
                            else
                              buildOption(option),
                        ],
                ),
              ),
            );
          },
        );
      },
    );
  }

  /// Chrome for the open menu: rounded on its outer end, square and strokeless
  /// at the join, so the trigger's border draws the divider.
  ShadDecoration _popoverDecoration(
    MixarThemeData theme,
    ShadThemeData shad, {
    required bool above,
  }) {
    final base = shad.popoverTheme.decoration ?? const ShadDecoration();
    final baseBorder = base.border;
    final edge =
        baseBorder?.bottom ??
        baseBorder?.top ??
        ShadBorderSide(
          color: theme.colors.border,
          width: theme.style.borderWidth,
        );
    final radius = theme.style.borderRadius.dialog.topLeft;
    return base.copyWith(
      border: ShadBorder(
        radius: above
            ? BorderRadius.vertical(top: radius)
            : BorderRadius.vertical(bottom: radius),
        top: above ? edge : ShadBorderSide.none,
        bottom: above ? ShadBorderSide.none : edge,
        left: baseBorder?.left ?? edge,
        right: baseBorder?.right ?? edge,
      ),
    );
  }

  ShadDecoration? _borderDecoration(
    MixarThemeData theme,
    BorderRadiusGeometry? radius, {
    Color? fill,
    bool join = false,
  }) {
    final color = widget.borderColor;
    if (color == null && radius == null && fill == null) return null;
    // ShadSelect sizes the popover from the trigger's width minus the trigger
    // decoration's padding, but the popover's 1px border still insets its
    // content. Match the trigger's horizontal padding to that border while
    // open so both boxes end at the same outer edge.
    final joinPadding = join
        ? EdgeInsets.symmetric(horizontal: theme.style.borderWidth)
        : null;
    if (color == null) {
      return ShadDecoration(
        color: fill,
        border: ShadBorder(radius: radius, padding: joinPadding),
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
      color: fill,
      border: ShadBorder(
        radius: radius,
        padding: joinPadding,
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
