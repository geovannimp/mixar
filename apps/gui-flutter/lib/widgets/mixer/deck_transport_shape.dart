import 'package:flutter/widgets.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';

/// Corner radius for the deck's Cue / Play transport chrome.
///
/// The transport is one pair of controls in two shapes: a wide bar under the
/// pads on a tall deck, and a square tile stacked beside them on a short one.
/// Both are buttons, so both wear the full pill.
BorderRadius deckTransportRadius(MixarThemeData theme) =>
    theme.style.borderRadius.pill;
