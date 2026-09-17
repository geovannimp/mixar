import 'dart:ui';

import 'package:flutter/foundation.dart';
import 'package:flutter/painting.dart';

/// Mixar color tokens (Forui-neutral visual parity).
@immutable
class MixarColors {
  const new({
    required this.brightness,
    required this.background,
    required this.foreground,
    required this.primary,
    required this.primaryForeground,
    required this.secondary,
    required this.secondaryForeground,
    required this.muted,
    required this.mutedForeground,
    required this.destructive,
    required this.destructiveForeground,
    required this.card,
    required this.border,
    this.hoverLighten = 0.075,
    this.hoverDarken = 0.05,
    this.disabledOpacity = 0.5,
  });

  /// Desktop light — values from Forui `FColors.neutralLight`.
  static const light = MixarColors(
    brightness: Brightness.light,
    background: Color(0xFFFFFFFF),
    foreground: Color(0xFF0A0A0A),
    primary: Color(0xFF171717),
    primaryForeground: Color(0xFFFAFAFA),
    secondary: Color(0xFFF5F5F5),
    secondaryForeground: Color(0xFF171717),
    muted: Color(0xFFF5F5F5),
    mutedForeground: Color(0xFF737373),
    destructive: Color(0xFFE7000B),
    destructiveForeground: Color(0xFFFAFAFA),
    card: Color(0xFFFFFFFF),
    border: Color(0xFFE5E5E5),
  );

  /// Desktop dark — values from Forui `FColors.neutralDark`.
  static const dark = MixarColors(
    brightness: Brightness.dark,
    background: Color(0xFF0A0A0A),
    foreground: Color(0xFFFAFAFA),
    primary: Color(0xFFE5E5E5),
    primaryForeground: Color(0xFF171717),
    secondary: Color(0xFF262626),
    secondaryForeground: Color(0xFFFAFAFA),
    muted: Color(0xFF262626),
    mutedForeground: Color(0xFFA1A1A1),
    destructive: Color(0xFFFF6467),
    destructiveForeground: Color(0xFFFAFAFA),
    card: Color(0xFF171717),
    border: Color(0x1AFFFFFF),
  );

  final Brightness brightness;
  final Color background;
  final Color foreground;
  final Color primary;
  final Color primaryForeground;
  final Color secondary;
  final Color secondaryForeground;
  final Color muted;
  final Color mutedForeground;
  final Color destructive;
  final Color destructiveForeground;
  final Color card;
  final Color border;
  final double hoverLighten;
  final double hoverDarken;
  final double disabledOpacity;

  /// Hovered variant (Forui `FColors.hover` algorithm).
  Color hover(Color color) {
    final hsl = HSLColor.fromColor(color);
    final l = hsl.lightness;
    final (space, factor, sign) = l > 0.5
        ? (1.0 - l, hoverDarken, -1)
        : (l, hoverLighten, 1);
    final aggressiveness = 1 + ((0.5 - space) / 0.5);
    final adjustment = factor * aggressiveness * sign;
    final lightness = clampDouble(l + adjustment, 0, 1);
    final hovered = hsl.withLightness(lightness).toColor();
    if (hovered.colorSpace != color.colorSpace) {
      return hovered.withValues(colorSpace: color.colorSpace);
    }
    return hovered;
  }

  /// Disabled variant (Forui `FColors.disable` algorithm).
  Color disable(Color color, [Color? background]) {
    final disabled = color.withValues(alpha: color.a * disabledOpacity);
    return background == null
        ? disabled
        : Color.alphaBlend(disabled, background);
  }
}
