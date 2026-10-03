import 'dart:ui';

import 'package:flutter/foundation.dart';
import 'package:flutter/painting.dart';

/// Mixar color tokens aligned with the website palette.
@immutable
class MixarColors {
  const MixarColors({
    required this.brightness,
    required this.background,
    required this.foreground,
    required this.primary,
    required this.primaryForeground,
    required this.secondary,
    required this.secondaryForeground,
    required this.muted,
    required this.mutedForeground,
    required this.accent,
    required this.destructive,
    required this.destructiveForeground,
    required this.card,
    required this.border,
    required this.selection,
    required this.selectionForeground,
    this.hoverLighten = 0.075,
    this.hoverDarken = 0.05,
    this.disabledOpacity = 0.5,
  });

  /// Desktop light — values from website `global.css` light theme.
  static const light = MixarColors(
    brightness: Brightness.light,
    background: Color(0xFFFAFAFA),
    foreground: Color(0xFF18181B),
    primary: Color(0xFF16A34A),
    primaryForeground: Color(0xFFFFFFFF),
    secondary: Color(0xFFF4F4F5),
    secondaryForeground: Color(0xFF171717),
    muted: Color(0xFFF4F4F5),
    mutedForeground: Color(0xFF52525B),
    accent: Color(0xFF0F766E),
    // Solid brand-green selection fill keeps dark ink legible.
    selection: Color(0xFF16A34A),
    selectionForeground: Color(0xFF09090B),
    destructive: Color(0xFFE7000B),
    destructiveForeground: Color(0xFFFAFAFA),
    card: Color(0xFFFFFFFF),
    border: Color(0x14000000),
  );

  /// Desktop dark — values from website `global.css` dark theme.
  static const dark = MixarColors(
    brightness: Brightness.dark,
    background: Color(0xFF09090B),
    foreground: Color(0xFFF4F4F5),
    primary: Color(0xFF4ADE80),
    primaryForeground: Color(0xFF09090B),
    secondary: Color(0xFF121216),
    secondaryForeground: Color(0xFFFAFAFA),
    muted: Color(0xFF121216),
    mutedForeground: Color(0xFFA1A1AA),
    accent: Color(0xFF2DD4BF),
    // Low-opacity brand wash: reads as a calm green highlight in a menu or
    // select without the neon glow of a solid `primary` block. Foreground is
    // the theme's regular text colour, which stays legible over the tint.
    selection: Color(0x294ADE80),
    selectionForeground: Color(0xFFF4F4F5),
    destructive: Color(0xFFFF6467),
    destructiveForeground: Color(0xFFFAFAFA),
    card: Color(0xFF18181C),
    border: Color(0x14FFFFFF),
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
  final Color accent;
  final Color destructive;
  final Color destructiveForeground;
  final Color card;
  final Color border;

  /// Fill for an active/hovered menu row, option row, or tab: a brand-green
  /// wash that stays legible under [selectionForeground].
  final Color selection;

  /// Text/icon colour drawn over [selection].
  final Color selectionForeground;
  final double hoverLighten;
  final double hoverDarken;
  final double disabledOpacity;

  /// Brand green dimmed toward the page background: a calmer green for filled
  /// accents (knob value arcs, play icon, selected-tab icon) that keeps the hue
  /// without the neon glow of raw [primary] in the dark theme.
  Color get primaryDim =>
      Color.alphaBlend(background.withValues(alpha: 0.45), primary);

  /// Brand green at low opacity: a subtle tint for filled surfaces (play
  /// button, selected tab) that reads as green without a solid accent block.
  Color get primaryTint => primary.withValues(alpha: 0.12);

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
