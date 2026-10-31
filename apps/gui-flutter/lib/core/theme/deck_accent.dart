import 'dart:ui';

import 'package:gui_flutter/core/theme/mixar_accent.dart';
import 'package:gui_flutter/core/theme/mixar_palette.dart';

/// Deck identity: the colour a deck's letters, badges, menu labels, jog ring,
/// knob arcs and fader grip all share.
///
/// This is theme territory, not fader territory — the fader adds its own lane
/// tint on top of a [deckTone] rather than owning the identity colour.
enum DeckAccent { a, b, c, d, neutral }

DeckAccent? deckAccentFor(int deckId) => switch (deckId) {
  0 => DeckAccent.a,
  1 => DeckAccent.b,
  2 => DeckAccent.c,
  3 => DeckAccent.d,
  _ => null,
};

/// Label ink for a deck, composed from its tone: the pieces stay separate so a
/// control can choose another rung, but every label reads this one derivation.
Color deckInk(DeckAccent accent, Brightness brightness) =>
    textColorStep(deckTone(accent), brightness);

String deckDisplayLabel(int deckId) => switch (deckId) {
  0 => 'Deck A',
  1 => 'Deck B',
  2 => 'Deck C',
  3 => 'Deck D',
  _ => 'Deck ${deckId + 1}',
};

/// One deck's full tone, mirroring the `Mixar*` ladders. Every consumer picks
/// its own rung — [textColorStep] for label ink, a fader its grip and lane tint
/// — so the choice stays visible where it is made instead of hiding inside a
/// single "deck colour" helper. There is no single canonical grip step.
MixarColorScale deckTone(DeckAccent accent) => switch (accent) {
  DeckAccent.a => _skyTone,
  DeckAccent.b => _roseTone,
  DeckAccent.c => _amberTone,
  DeckAccent.d => _violetTone,
  DeckAccent.neutral => _zincTone,
};

const _skyTone = MixarColorScale(
  s50: MixarSky.s50,
  s100: MixarSky.s100,
  s200: MixarSky.s200,
  s300: MixarSky.s300,
  s400: MixarSky.s400,
  s500: MixarSky.s500,
  s600: MixarSky.s600,
  s700: MixarSky.s700,
  s800: MixarSky.s800,
  s900: MixarSky.s900,
  s950: MixarSky.s950,
);

const _roseTone = MixarColorScale(
  s50: MixarRose.s50,
  s100: MixarRose.s100,
  s200: MixarRose.s200,
  s300: MixarRose.s300,
  s400: MixarRose.s400,
  s500: MixarRose.s500,
  s600: MixarRose.s600,
  s700: MixarRose.s700,
  s800: MixarRose.s800,
  s900: MixarRose.s900,
  s950: MixarRose.s950,
);

const _amberTone = MixarColorScale(
  s50: MixarAmber.s50,
  s100: MixarAmber.s100,
  s200: MixarAmber.s200,
  s300: MixarAmber.s300,
  s400: MixarAmber.s400,
  s500: MixarAmber.s500,
  s600: MixarAmber.s600,
  s700: MixarAmber.s700,
  s800: MixarAmber.s800,
  s900: MixarAmber.s900,
  s950: MixarAmber.s950,
);

const _violetTone = MixarColorScale(
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

const _zincTone = MixarColorScale(
  s50: MixarZinc.s50,
  s100: MixarZinc.s100,
  s200: MixarZinc.s200,
  s300: MixarZinc.s300,
  s400: MixarZinc.s400,
  s500: MixarZinc.s500,
  s600: MixarZinc.s600,
  s700: MixarZinc.s700,
  s800: MixarZinc.s800,
  s900: MixarZinc.s900,
  s950: MixarZinc.s950,
);
