import 'package:flutter/foundation.dart';
import 'package:flutter/painting.dart';
import 'package:gui_flutter/core/theme/mixar_palette.dart';

/// One 11-step colour ladder; the shade names mirror the Tailwind steps the
/// palette and deck tones are built from. Callers pick the step they need, so
/// one tone can serve a bright grip, a deep lane tint and a label.
@immutable
class MixarColorScale {
  const MixarColorScale({
    required this.s50,
    required this.s100,
    required this.s200,
    required this.s300,
    required this.s400,
    required this.s500,
    required this.s600,
    required this.s700,
    required this.s800,
    required this.s900,
    required this.s950,
  });

  final Color s50;
  final Color s100;
  final Color s200;
  final Color s300;
  final Color s400;
  final Color s500;
  final Color s600;
  final Color s700;
  final Color s800;
  final Color s900;
  final Color s950;
}

/// Text and control ink for a tone on this brightness: the 400 rung on the
/// dark plate, the 800 rung on warm bone. The 600 is skipped because on bone-100
/// sky-600 measures 3.53:1 and amber-600 2.74:1, so a label in a deck's identity
/// rung fails AA. This is a contrast rule, not an offset from any other control:
/// ink is lighter than the dark fader grip and several rungs darker than the
/// light one. Bright fills and deep tints stay a call-site choice.
Color textColorStep(MixarColorScale color, Brightness brightness) =>
    brightness == Brightness.dark ? color.s400 : color.s800;

/// Accent palettes a user can pick in Settings → UI → Chrome. Teal is the
/// brand default (website `--accent` family); the rest are Tailwind ladders
/// so each palette keeps a full light/dark range.
enum MixarAccentPalette {
  teal,
  blue,
  violet,
  orange,
  pink;

  MixarColorScale get scale => switch (this) {
    MixarAccentPalette.teal => _teal,
    MixarAccentPalette.blue => _blue,
    MixarAccentPalette.violet => _violet,
    MixarAccentPalette.orange => _orange,
    MixarAccentPalette.pink => _pink,
  };
}

const _teal = MixarColorScale(
  s50: MixarTeal.s50,
  s100: MixarTeal.s100,
  s200: MixarTeal.s200,
  s300: MixarTeal.s300,
  s400: MixarTeal.s400,
  s500: MixarTeal.s500,
  s600: MixarTeal.s600,
  s700: MixarTeal.s700,
  s800: MixarTeal.s800,
  s900: MixarTeal.s900,
  s950: MixarTeal.s950,
);

const _blue = MixarColorScale(
  s50: MixarBlue.s50,
  s100: MixarBlue.s100,
  s200: MixarBlue.s200,
  s300: MixarBlue.s300,
  s400: MixarBlue.s400,
  s500: MixarBlue.s500,
  s600: MixarBlue.s600,
  s700: MixarBlue.s700,
  s800: MixarBlue.s800,
  s900: MixarBlue.s900,
  s950: MixarBlue.s950,
);

const _violet = MixarColorScale(
  s50: MixarViolet.s50,
  s100: MixarViolet.s100,
  s200: MixarViolet.s200,
  s300: MixarViolet.s300,
  s400: MixarViolet.s400,
  s500: MixarViolet.s500,
  s600: MixarViolet.s600,
  s700: MixarViolet.s700,
  s800: MixarViolet.s800,
  s900: MixarViolet.s900,
  s950: MixarViolet.s950,
);

const _orange = MixarColorScale(
  s50: MixarOrange.s50,
  s100: MixarOrange.s100,
  s200: MixarOrange.s200,
  s300: MixarOrange.s300,
  s400: MixarOrange.s400,
  s500: MixarOrange.s500,
  s600: MixarOrange.s600,
  s700: MixarOrange.s700,
  s800: MixarOrange.s800,
  s900: MixarOrange.s900,
  s950: MixarOrange.s950,
);

const _pink = MixarColorScale(
  s50: MixarPink.s50,
  s100: MixarPink.s100,
  s200: MixarPink.s200,
  s300: MixarPink.s300,
  s400: MixarPink.s400,
  s500: MixarPink.s500,
  s600: MixarPink.s600,
  s700: MixarPink.s700,
  s800: MixarPink.s800,
  s900: MixarPink.s900,
  s950: MixarPink.s950,
);
