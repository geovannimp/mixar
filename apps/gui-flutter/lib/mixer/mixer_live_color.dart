import 'package:flutter/widgets.dart';

/// Solid emerald-400 — same hue as [MixarColors.dark.primary] / website
/// `--deck-a` (dark). Booth chrome uses the alpha variants below.
const kMixerEmerald = Color(0xFF34D399);

/// Emerald “live/on” signal shared by headphone cue, quantize, and deck drop
/// hover — keep these in lockstep so booth chrome reads as one system.
const kMixerLiveOn = Color(0xbf34d399); // emerald-400 @ ~75%
const kMixerLiveFill = Color(0x1c34d399); // emerald-400 @ ~11%

/// Cue amber — deck Cue + master PFL (DESIGN.md cue amber).
const kMixerCueOn = Color(0xFFFCD34D); // amber-300
const kMixerCueBorder = Color(0x66F59E0B); // amber-500/40
const kMixerCueFill = Color(0x14F59E0B); // amber-500/8 — soft audition wash
