import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';
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
    expect(seeded.primary, MixarBlue.s600);
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
    // The deep wash stays magenta (Tailwind pink-950 sat at hue 336 = maroon).
    expect(
      HSLColor.fromColor(seeded.selection).hue,
      inInclusiveRange(280, 320),
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
}
