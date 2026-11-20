import 'package:flutter/widgets.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';

/// Corner radius for the deck's Cue / Play transport chrome.
///
/// The transport is one pair of controls in two shapes: a wide bar under the
/// pads on a tall deck, and a square tile stacked beside them on a short one.
/// Only the square tile is fully rounded — the wide bar keeps the theme's
/// square chrome, which is what the transport row has always looked like.
BorderRadius deckTransportRadius(
  MixarThemeData theme,
  BoxConstraints constraints,
) {
  final box = _isSquareTile(constraints);
  return box ? theme.style.borderRadius.pill : theme.style.borderRadius.md;
}

/// True while the control reads as a tile rather than a bar. The two shapes
/// sit far apart (a stacked tile is within ~1.2 of square, the wide row is
/// several times wider than it is tall), so a loose threshold is enough and
/// sub-pixel layout noise cannot flip it.
bool _isSquareTile(BoxConstraints constraints) {
  if (!constraints.hasBoundedWidth || !constraints.hasBoundedHeight) {
    return false;
  }
  return constraints.maxWidth <= constraints.maxHeight * 1.5;
}
