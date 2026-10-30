import 'package:gui_flutter/core/theme/mixar_palette.dart';

/// Cue amber border/label when theme is unavailable — prefer
/// [MixarColors.cueOn] in widgets (theme-aware for light/dark contrast).
const kMixerCueOn = MixarAmber.s300;
const kMixerCueBorder = MixarAmber.s600;

/// Master crown selected icon (gold) — header set-master chrome.
const kMixerMasterOn = MixarAmber.s300;

/// Soft gold wash behind selected master chrome (~19% amber-400).
final kMixerMasterFill = MixarAmber.s400.withValues(alpha: 0.19);
