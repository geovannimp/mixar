import 'package:flutter/widgets.dart';
import 'package:gui_flutter/shell/mixar_palette.dart';

/// Teal “live/on” signal shared by headphone cue, quantize, and deck drop
/// hover — keep these in lockstep so booth chrome reads as one system.
/// Solid teal-400 (same as MixarColors.dark.primary / website `--accent`).
const kMixerLiveOn = MixarTeal.s400;

/// Cue amber — deck Cue + master PFL (DESIGN.md cue amber).
const kMixerCueOn = MixarAmber.s300;
const kMixerCueBorder = MixarAmber.s600;
