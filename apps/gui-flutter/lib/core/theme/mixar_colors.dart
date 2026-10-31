// GENERATED FILE — do not edit.
//
// Source: tokens/semantic.json. Regenerate with `npm run tokens:build`;
// `npm run tokens:check` fails CI when this file and the tokens disagree.

import 'dart:ui';

import 'package:flutter/foundation.dart';
import 'package:flutter/painting.dart';
import 'package:gui_flutter/core/theme/mixar_accent.dart';
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
    required this.primaryHover,
    required this.destructive,
    required this.destructiveForeground,
    required this.card,
    required this.waveformBackground,
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

  /// Desktop light — warm bone paper, mirrored by website `global.css` light
  /// theme. Surfaces climb the bone ladder (`background` s100, `card` s50), so
  /// no panel is pure white and raised planes read apart from the page.
  static const light = MixarColors(
    brightness: Brightness.light,
    background: MixarBone.s100,
    foreground: MixarBone.s900,
    primary: MixarTeal.s700,
    primaryForeground: Color(0xFFFFFFFF),
    secondary: MixarBone.s200,
    secondaryForeground: MixarBone.s900,
    muted: MixarBone.s200,
    mutedForeground: MixarBone.s600,
    accent: MixarTeal.s700,
    primaryHover: MixarTeal.s800,
    destructive: Color(0xFFE7000B),
    destructiveForeground: MixarBone.s50,
    card: MixarBone.s50,
    waveformBackground: MixarBone.s200,
    border: MixarBone.s300,
    selection: MixarTeal.s100,
    selectionForeground: MixarZinc.s950,
    primaryTint: MixarTeal.s50,
    primaryDim: MixarTeal.s500,
    accentTint: MixarTeal.s50,
    cueTint: MixarAmber.s100,
    cueOn: MixarAmber.s700,
  );

  /// Desktop dark — booth chrome, mirrored by the website dark theme.
  static const dark = MixarColors(
    brightness: Brightness.dark,
    background: MixarZinc.s950,
    foreground: MixarZinc.s100,
    primary: MixarTeal.s400,
    primaryForeground: MixarZinc.s950,
    secondary: Color(0xFF121216),
    secondaryForeground: MixarZinc.s50,
    muted: Color(0xFF121216),
    mutedForeground: MixarZinc.s400,
    accent: MixarTeal.s400,
    primaryHover: MixarTeal.s300,
    destructive: Color(0xFFFF6467),
    destructiveForeground: MixarZinc.s950,
    card: Color(0xFF18181C),
    waveformBackground: Color(0xFF050508),
    border: MixarZinc.s800,
    selection: MixarTeal.s950,
    selectionForeground: MixarZinc.s100,
    primaryTint: MixarTeal.s950,
    primaryDim: MixarTeal.s600,
    accentTint: MixarTeal.s950,
    cueTint: MixarAmber.s950,
    cueOn: MixarAmber.s300,
  );

  final Brightness brightness;
  final Color background;
  final Color foreground;

  /// Fill + text + link. Teal-600 measured 3.22:1 on bone-100 as text and
  /// 3.74:1 white-on-fill — both fail WCAG AA; teal-700 measures 4.71:1 and
  /// 5.47:1. One step darker is the accessibility floor, not a taste call.
  final Color primary;
  final Color primaryForeground;
  final Color secondary;
  final Color secondaryForeground;
  final Color muted;
  final Color mutedForeground;

  /// Website `--accent` / app `accent`: same teal ladder as `primary`.
  final Color accent;

  /// Hover/press fill for primary buttons and links: one rung darker than
  /// `primary` on warm bone (lightening a dark fill washes the white label
  /// out).
  final Color primaryHover;
  final Color destructive;
  final Color destructiveForeground;
  final Color card;

  /// Waveform lane fill the spectral bars are drawn over. Baked into recorded
  /// strip pictures, so it travels with the strip cache key and re-records when
  /// the theme flips.
  final Color waveformBackground;
  final Color border;

  /// Soft solid selection — same deeper fill for tabs, library rows, and menus
  /// so selected chrome stays quiet.
  final Color selection;
  final Color selectionForeground;
  final Color primaryTint;
  final Color primaryDim;
  final Color accentTint;
  final Color cueTint;

  /// Dark amber over the cream cueTint — s300 on s100 fails WCAG.
  final Color cueOn;
  final double hoverLighten;
  final double hoverDarken;
  final double disabledOpacity;

  /// Accent-seeded copy: same neutrals, cue amber and hairlines, brand tokens
  /// re-read from [scale]. Shade slots are fixed per brightness, so every
  /// palette keeps the contrast the teal default was tuned for.
  MixarColors withAccentScale(MixarColorScale scale) {
    final dark = brightness == Brightness.dark;
    return MixarColors(
      brightness: brightness,
      background: background,
      foreground: foreground,
      primary: dark ? scale.s400 : scale.s700,
      primaryForeground: primaryForeground,
      secondary: secondary,
      secondaryForeground: secondaryForeground,
      muted: muted,
      mutedForeground: mutedForeground,
      accent: dark ? scale.s400 : scale.s700,
      primaryHover: dark ? scale.s300 : scale.s800,
      destructive: destructive,
      destructiveForeground: destructiveForeground,
      card: card,
      waveformBackground: waveformBackground,
      border: border,
      selection: dark ? scale.s950 : scale.s100,
      selectionForeground: selectionForeground,
      primaryTint: dark ? scale.s950 : scale.s50,
      primaryDim: dark ? scale.s600 : scale.s500,
      accentTint: dark ? scale.s950 : scale.s50,
      cueTint: cueTint,
      cueOn: cueOn,
      hoverLighten: hoverLighten,
      hoverDarken: hoverDarken,
      disabledOpacity: disabledOpacity,
    );
  }

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
