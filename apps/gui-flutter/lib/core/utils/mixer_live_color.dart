import 'package:gui_flutter/core/theme/mixar_palette.dart';

/// Teal “live/on” signal shared by headphone cue, quantize, and deck drop
/// hover — keep these in lockstep so booth chrome reads as one system.
/// Solid teal-400 (same as MixarColors.dark.primary / website `--accent`).
const kMixerLiveOn = MixarTeal.s400;

/// Cue amber border/label when theme is unavailable — prefer
/// [MixarColors.cueOn] in widgets (theme-aware for light/dark contrast).
const kMixerCueOn = MixarAmber.s300;
const kMixerCueBorder = MixarAmber.s600;

/// Master crown selected icon (gold) — header set-master chrome.
const kMixerMasterOn = MixarAmber.s300;

/// Soft gold wash behind selected master chrome (~19% amber-400).
final kMixerMasterFill = MixarAmber.s400.withValues(alpha: 0.19);
