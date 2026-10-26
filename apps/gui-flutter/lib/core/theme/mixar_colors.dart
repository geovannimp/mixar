import 'dart:ui';

import 'package:flutter/foundation.dart';
import 'package:flutter/painting.dart';
import 'package:gui_flutter/core/theme/mixar_palette.dart';

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
    required this.primaryTint,
    required this.primaryDim,
    required this.accentTint,
    required this.cueTint,
    required this.cueOn,
    this.hoverLighten = 0.075,
    this.hoverDarken = 0.05,
    this.disabledOpacity = 0.5,
  });

  /// Desktop light — values from website `global.css` light theme.
  static const light = MixarColors(
    brightness: Brightness.light,
    background: Color(0xFFFAFAFA),
    foreground: Color(0xFF18181B),
    // One teal ladder for brand primary + accent (website `--accent` family).
    primary: MixarTeal.s600,
    primaryForeground: Color(0xFFFFFFFF),
    secondary: Color(0xFFF4F4F5),
    secondaryForeground: Color(0xFF171717),
    muted: Color(0xFFF4F4F5),
    mutedForeground: Color(0xFF52525B),
    accent: MixarTeal.s700,
    selection: MixarTeal.s100,
    selectionForeground: Color(0xFF09090B),
    primaryTint: MixarTeal.s50,
    primaryDim: MixarTeal.s500,
    accentTint: MixarTeal.s50,
    cueTint: MixarAmber.s100,
    // Dark amber over cream cueTint — s300 on s100 fails WCAG.
    cueOn: MixarAmber.s700,
    destructive: Color(0xFFE7000B),
    destructiveForeground: Color(0xFFFAFAFA),
    card: Color(0xFFFFFFFF),
    border: MixarZinc.s200,
  );

  /// Desktop dark — values from website `global.css` dark theme.
  static const dark = MixarColors(
    brightness: Brightness.dark,
    background: Color(0xFF09090B),
    foreground: Color(0xFFF4F4F5),
    primary: MixarTeal.s400,
    primaryForeground: Color(0xFF09090B),
    secondary: Color(0xFF121216),
    secondaryForeground: Color(0xFFFAFAFA),
    muted: Color(0xFF121216),
    mutedForeground: Color(0xFFA1A1AA),
    accent: MixarTeal.s400,
    // Soft solid selection — teal-950. Same deeper fill for tabs, library
    // rows, and menus so selected chrome stays quiet against the booth.
    selection: MixarTeal.s950,
    selectionForeground: Color(0xFFF4F4F5),
    primaryTint: MixarTeal.s950,
    // Calmer mid teal in dark — s500 reads too neon on booth chrome.
    primaryDim: MixarTeal.s600,
    accentTint: MixarTeal.s950,
    cueTint: MixarAmber.s950,
    cueOn: MixarAmber.s300,
    destructive: Color(0xFFFF6467),
    destructiveForeground: Color(0xFFFAFAFA),
    card: Color(0xFF18181C),
    border: MixarZinc.s800,
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

  /// Hairline colour — a solid palette step, never an alpha wash.
  ///
  /// Translucent borders change shade with whatever sits behind them, so two
  /// lines that should match (panel frame, header divider) drift apart. Pinning
  /// the step keeps every border the same colour on every surface.
  final Color border;

  /// Soft selected-row / menu-option fill — solid teal step, surface-stable.
  final Color selection;

  /// Text/icon colour drawn over [selection].
  final Color selectionForeground;

  /// Soft teal fill (mapping "on", Q wash) — same step as [selection] in dark.
  final Color primaryTint;

  /// Mid teal for calmer accents (knob arcs) without raw [primary] neon.
  final Color primaryDim;

  /// Softest teal fill (Play while playing).
  final Color accentTint;

  /// Softest amber fill (Cue while auditioning).
  final Color cueTint;

  /// Amber label/border over cue chrome — darker in light for contrast on
  /// [cueTint], brighter in dark for outline transport.
  final Color cueOn;

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
