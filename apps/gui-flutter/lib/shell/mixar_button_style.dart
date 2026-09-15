import 'package:flutter/widgets.dart';
import 'package:forui/forui.dart';

import 'package:gui_flutter/shell/m_tappable.dart';

enum MixarButtonVariant { primary, secondary, outline, ghost, destructive }

enum MixarButtonSize { xs, sm, md }

enum MixarButtonDensity {
  /// Deck / mixer chrome — slightly tighter type.
  mixer,

  /// Settings, library, shell, dialogs.
  app,
}

@immutable
class MixarButtonLook {
  const MixarButtonLook({
    required this.decoration,
    required this.foreground,
    required this.textStyle,
    required this.iconSize,
    required this.contentConstraints,
    required this.contentPadding,
    required this.iconPadding,
    required this.contentSpacing,
    required this.showFocusOutline,
    required this.focusColor,
    required this.borderRadius,
  });

  final BoxDecoration decoration;
  final Color foreground;
  final TextStyle textStyle;
  final double iconSize;
  final BoxConstraints contentConstraints;
  final EdgeInsetsGeometry contentPadding;
  final EdgeInsetsGeometry iconPadding;
  final double contentSpacing;
  final bool showFocusOutline;
  final Color focusColor;
  final BorderRadius borderRadius;
}

/// Forui-desktop-parity button look from [FTheme], without Forui widgets.
MixarButtonLook resolveMixarButtonLook({
  required FThemeData theme,
  required MixarButtonVariant variant,
  required MixarButtonSize size,
  required MixarButtonDensity density,
  required MTappableState state,
  required bool icon,
  EdgeInsetsGeometry? padding,
  Color? backgroundColor,
}) {
  final colors = theme.colors;
  final style = theme.style;
  final light = colors.brightness == Brightness.light;
  final radius = switch (size) {
    MixarButtonSize.xs => style.borderRadius.sm,
    MixarButtonSize.sm || MixarButtonSize.md => style.borderRadius.md,
  };

  final (
    Color fill,
    Color? border,
    Color fg,
    Color disabledFg,
  ) = switch (variant) {
    MixarButtonVariant.primary => (
      _fill(
        colors.primary,
        colors.hover(colors.primary),
        colors.disable(colors.primary),
        state,
      ),
      null,
      state.disabled
          ? colors.disable(colors.primaryForeground)
          : colors.primaryForeground,
      colors.disable(colors.primaryForeground),
    ),
    MixarButtonVariant.secondary => (
      _fill(
        colors.secondary,
        colors.hover(colors.secondary),
        colors.disable(colors.secondary),
        state,
      ),
      null,
      state.disabled
          ? colors.disable(colors.secondaryForeground)
          : colors.secondaryForeground,
      colors.disable(colors.secondaryForeground),
    ),
    MixarButtonVariant.destructive => (
      _fill(
        colors.destructive.withValues(alpha: light ? 0.1 : 0.2),
        colors.destructive.withValues(alpha: light ? 0.2 : 0.3),
        colors.destructive.withValues(alpha: light ? 0.05 : 0.1),
        state,
      ),
      null,
      state.disabled
          ? colors.destructive.withValues(alpha: 0.5)
          : colors.destructive,
      colors.destructive.withValues(alpha: 0.5),
    ),
    MixarButtonVariant.outline => (
      _fill(colors.card, colors.secondary, colors.disable(colors.card), state),
      colors.border,
      state.disabled
          ? colors.disable(colors.secondaryForeground)
          : colors.secondaryForeground,
      colors.disable(colors.secondaryForeground),
    ),
    MixarButtonVariant.ghost => (
      state.active
          ? (state.disabled
                ? colors.disable(colors.secondary)
                : colors.secondary)
          : const Color(0x00000000),
      null,
      state.disabled
          ? colors.disable(colors.secondaryForeground)
          : colors.secondaryForeground,
      colors.disable(colors.secondaryForeground),
    ),
  };

  final resolvedFill = backgroundColor ?? fill;
  final metrics = _metrics(theme: theme, size: size, density: density);
  final weight = density == MixarButtonDensity.mixer
      ? FontWeight.w600
      : FontWeight.w500;
  final letterSpacing = density == MixarButtonDensity.mixer ? 0.2 : 0.0;

  return MixarButtonLook(
    decoration: BoxDecoration(
      color: resolvedFill,
      border: border == null
          ? null
          : Border.all(color: border, width: style.borderWidth),
      borderRadius: radius,
    ),
    foreground: state.disabled ? disabledFg : fg,
    textStyle: metrics.textStyle.copyWith(
      color: state.disabled ? disabledFg : fg,
      fontWeight: weight,
      height: 1,
      letterSpacing: letterSpacing,
    ),
    iconSize: metrics.iconSize,
    contentConstraints: metrics.contentConstraints,
    contentPadding: padding ?? metrics.contentPadding,
    iconPadding: padding ?? metrics.iconPadding,
    contentSpacing: metrics.contentSpacing,
    showFocusOutline: state.focused && !state.disabled,
    focusColor: colors.primary,
    borderRadius: radius,
  );
}

Color _fill(Color idle, Color active, Color disabled, MTappableState state) {
  if (state.disabled) {
    return disabled;
  }
  if (state.active) {
    return active;
  }
  return idle;
}

({
  TextStyle textStyle,
  double iconSize,
  BoxConstraints contentConstraints,
  EdgeInsetsGeometry contentPadding,
  EdgeInsetsGeometry iconPadding,
  double contentSpacing,
})
_metrics({
  required FThemeData theme,
  required MixarButtonSize size,
  required MixarButtonDensity density,
}) {
  // Forui non-touch desktop metrics.
  final base = switch (size) {
    MixarButtonSize.xs => (
      textStyle: theme.typography.body.xs,
      iconSize: theme.typography.body.sm.fontSize ?? 14,
      contentConstraints: const BoxConstraints(minWidth: 24, minHeight: 24),
      contentPadding: const EdgeInsets.symmetric(horizontal: 8, vertical: 6),
      iconPadding: const EdgeInsets.all(5),
      contentSpacing: 4.0,
    ),
    MixarButtonSize.sm => (
      textStyle: theme.typography.body.sm,
      iconSize: theme.typography.body.md.fontSize ?? 16,
      contentConstraints: const BoxConstraints(minWidth: 32, minHeight: 32),
      contentPadding: const EdgeInsets.symmetric(horizontal: 10, vertical: 9),
      iconPadding: const EdgeInsets.all(8),
      contentSpacing: 4.0,
    ),
    MixarButtonSize.md => (
      textStyle: theme.typography.body.sm,
      iconSize: theme.typography.body.md.fontSize ?? 16,
      contentConstraints: const BoxConstraints(minWidth: 36, minHeight: 36),
      contentPadding: const EdgeInsets.symmetric(horizontal: 10, vertical: 11),
      iconPadding: const EdgeInsets.all(10),
      contentSpacing: 6.0,
    ),
  };
  if (density != MixarButtonDensity.mixer || size != MixarButtonSize.xs) {
    return base;
  }
  // Mixer xs: keep height, shave a hair of horizontal padding.
  return (
    textStyle: base.textStyle,
    iconSize: base.iconSize,
    contentConstraints: base.contentConstraints,
    contentPadding: const EdgeInsets.symmetric(horizontal: 7, vertical: 6),
    iconPadding: base.iconPadding,
    contentSpacing: base.contentSpacing,
  );
}
