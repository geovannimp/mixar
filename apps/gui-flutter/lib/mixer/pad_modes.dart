import 'package:flutter/services.dart';
import 'package:gui_flutter/src/rust/api/engine.dart' show PadMode;

export 'package:gui_flutter/src/rust/api/engine.dart' show PadMode;

/// Pad mode helpers + Tauri-matching beat tables / labels.

const kPadModes = <PadMode>[
  PadMode.hotCue,
  PadMode.loopRoll,
  PadMode.beatJump,
  PadMode.sampler,
];

const kLoopRollBeats = <num>[1 / 32, 1 / 16, 1 / 8, 1 / 4, 1 / 2, 1, 2, 4];
const kBeatJumpForward = <num>[1, 2, 4, 8, 16, 32, 64, 128];
const kBeatJumpBack = <num>[-1, -2, -4, -8, -16, -32, -64, -128];

/// Sampler play-mode wire values (Tauri `SamplerPlayMode`).
const kSamplerPlayModeOneshot = 'oneshot';
const kSamplerPlayModeHold = 'hold';
const kSamplerPlayModeLoop = 'loop';

/// Default sampler play mode (Tauri `DEFAULT_SAMPLER_PLAY_MODE`).
const kDefaultSamplerPlayMode = kSamplerPlayModeOneshot;

/// Bank settings dialog options (`default` = inherit settings).
const kSamplerPlayModeOptions = <String>[
  'default',
  kSamplerPlayModeOneshot,
  kSamplerPlayModeHold,
  kSamplerPlayModeLoop,
];

String padModeShortLabel(PadMode mode) => switch (mode) {
  PadMode.hotCue => 'Cue',
  PadMode.loopRoll => 'Roll',
  PadMode.beatJump => 'Jump',
  PadMode.sampler => 'Sample',
};

PadMode cyclePadMode(PadMode mode, int direction) {
  final index = kPadModes.indexOf(mode);
  final current = index >= 0 ? index : 0;
  final len = kPadModes.length;
  final next = direction < 0 ? (current + len - 1) % len : (current + 1) % len;
  return kPadModes[next];
}

bool shiftKeyPressed() =>
    HardwareKeyboard.instance.isLogicalKeyPressed(
      LogicalKeyboardKey.shiftLeft,
    ) ||
    HardwareKeyboard.instance.isLogicalKeyPressed(
      LogicalKeyboardKey.shiftRight,
    );
