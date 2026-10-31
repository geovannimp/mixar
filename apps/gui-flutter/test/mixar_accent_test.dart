import 'dart:math' as math;

import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/theme/deck_accent.dart';
import 'package:gui_flutter/core/theme/mixar_accent.dart';
import 'package:gui_flutter/core/theme/mixar_colors.dart';
import 'package:gui_flutter/core/theme/mixar_palette.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';

void main() {
  test('teal scale reproduces the stock dark palette', () {
    final seeded = MixarColors.dark.withAccentScale(
      MixarAccentPalette.teal.scale,
    );
    const stock = MixarColors.dark;
    for (final (name, seededColor, stockColor) in [
      ('primary', seeded.primary, stock.primary),
      ('accent', seeded.accent, stock.accent),
      ('primaryHover', seeded.primaryHover, stock.primaryHover),
      ('selection', seeded.selection, stock.selection),
      ('primaryTint', seeded.primaryTint, stock.primaryTint),
      ('primaryDim', seeded.primaryDim, stock.primaryDim),
      ('accentTint', seeded.accentTint, stock.accentTint),
      (
        'selectionForeground',
        seeded.selectionForeground,
        stock.selectionForeground,
      ),
      ('background', seeded.background, stock.background),
      ('border', seeded.border, stock.border),
      ('card', seeded.card, stock.card),
      // Baked into recorded waveform strip pictures, so a palette must never
      // touch it or the strip cache would keep stale lanes.
      (
        'waveformBackground',
        seeded.waveformBackground,
        stock.waveformBackground,
      ),
      ('cueTint', seeded.cueTint, stock.cueTint),
      ('cueOn', seeded.cueOn, stock.cueOn),
    ]) {
      expect(seededColor, stockColor, reason: name);
    }
  });

  test('light palette swaps brand slots but keeps neutrals', () {
    final seeded = MixarColors.light.withAccentScale(
      MixarAccentPalette.blue.scale,
    );
    expect(seeded.primary, MixarBlue.s700);
    expect(seeded.accent, MixarBlue.s700);
    expect(seeded.selection, MixarBlue.s100);
    expect(seeded.primaryTint, MixarBlue.s50);
    expect(seeded.accentTint, MixarBlue.s50);
    expect(seeded.primaryDim, MixarBlue.s500);
    expect(seeded.border, MixarColors.light.border);
    expect(seeded.background, MixarColors.light.background);
    expect(seeded.cueOn, MixarColors.light.cueOn);
  });

  test('dark palette reads the 400 step for primary and accent', () {
    final seeded = MixarColors.dark.withAccentScale(
      MixarAccentPalette.pink.scale,
    );
    expect(seeded.primary, MixarPink.s400);
    expect(seeded.accent, MixarPink.s400);
    expect(seeded.selection, MixarPink.s950);
    // The deep wash must stay blue-leaning (magenta), not red-leaning: the
    // shipped Tailwind pink-950 `#500724` is R>B and reads maroon.
    expect(
      seeded.selection.b,
      greaterThan(seeded.selection.r),
      reason: 'pink selection wash stays magenta instead of drifting to red',
    );
    expect(seeded.background, MixarColors.dark.background);
  });

  test('themes are memoised per palette and brightness', () {
    final violet = MixarThemeData.of(
      MixarAccentPalette.violet,
      Brightness.dark,
    );
    expect(violet.colors.primary, MixarViolet.s400);
    expect(violet.colors.selection, MixarViolet.s950);
    expect(
      MixarThemeData.of(MixarAccentPalette.teal, Brightness.dark),
      same(MixarThemeData.dark()),
    );
    expect(
      MixarThemeData.of(MixarAccentPalette.violet, Brightness.dark),
      same(violet),
    );
    expect(
      MixarThemeData.of(MixarAccentPalette.violet, Brightness.light),
      isNot(same(violet)),
    );
    expect(
      MixarThemeData.of(MixarAccentPalette.pink, Brightness.dark),
      isNot(same(violet)),
    );
  });

  /// WCAG 2.x relative luminance and contrast ratio, so the palette rules below
  /// are asserted as ratios rather than as pinned hex values.
  double _luminance(Color color) {
    double channel(double c) => c <= 0.03928
        ? c / 12.92
        : math.pow((c + 0.055) / 1.055, 2.4).toDouble();
    return 0.2126 * channel(color.r) +
        0.7152 * channel(color.g) +
        0.0722 * channel(color.b);
  }

  double _contrast(Color a, Color b) {
    final la = _luminance(a);
    final lb = _luminance(b);
    return (math.max(la, lb) + 0.05) / (math.min(la, lb) + 0.05);
  }

  test('deck ink clears AA on both plates', () {
    for (final accent in DeckAccent.values) {
      final tone = deckTone(accent);
      for (final brightness in Brightness.values) {
        final background = brightness == Brightness.light
            ? MixarColors.light.background
            : MixarColors.dark.background;
        expect(
          _contrast(textColorStep(tone, brightness), background),
          greaterThanOrEqualTo(4.5),
          reason: '$accent ink on the ${brightness.name} plate',
        );
      }
    }
  });

  test('primary hover keeps its label readable', () {
    for (final colors in [MixarColors.light, MixarColors.dark]) {
      expect(
        _contrast(colors.primaryHover, colors.primaryForeground),
        greaterThanOrEqualTo(4.5),
        reason: '${colors.brightness} primary hover label',
      );
    }
  });

  test('destructive text inside a destructive fill clears AA', () {
    for (final colors in [MixarColors.light, MixarColors.dark]) {
      expect(
        _contrast(colors.destructiveForeground, colors.destructive),
        greaterThanOrEqualTo(4.5),
        reason: '${colors.brightness} destructive toast label',
      );
    }
  });
}
