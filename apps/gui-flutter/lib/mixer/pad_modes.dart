import 'package:flutter/services.dart';
import 'package:gui_flutter/src/rust/api/engine.dart'
    show KeyboardScale, PadMode;

export 'package:gui_flutter/src/rust/api/engine.dart'
    show KeyboardScale, PadMode;

/// Pad mode helpers + Tauri-matching beat tables / labels.

const kPadModes = <PadMode>[
  PadMode.hotCue,
  PadMode.loopRoll,
  PadMode.beatJump,
  PadMode.sampler,
  PadMode.stems,
  PadMode.keyboard,
  PadMode.keyShift,
];

/// Key Shift pad semitone offsets for slots 0..=7 (matches engine
/// `KEY_SHIFT_PAD_SEMITONES`).
const kKeyShiftPadSemitones = <int>[0, 1, 2, 3, -4, -3, -2, -1];

/// Keyboard pad scale degrees (semitones from the analyzed root) per scale
/// (matches engine `keyboard_scale_degrees`).
const kKeyboardScaleDegrees = <KeyboardScale, List<int>>{
  KeyboardScale.major: [0, 2, 4, 5, 7, 9, 11, 12],
  KeyboardScale.minor: [0, 2, 3, 5, 7, 8, 10, 12],
  KeyboardScale.pentatonic: [0, 2, 4, 7, 9, 12, 14, 16],
};

const kLoopRollBeats = <num>[1 / 32, 1 / 16, 1 / 8, 1 / 4, 1 / 2, 1, 2, 4];
const kBeatJumpForward = <num>[1, 2, 4, 8, 16, 32, 64, 128];
const kBeatJumpBack = <num>[-1, -2, -4, -8, -16, -32, -64, -128];

/// Sampler play-mode wire values (Tauri `SamplerPlayMode`).
const kSamplerPlayModeOneshot = 'oneshot';
const kSamplerPlayModeHold = 'hold';
const kSamplerPlayModeLoop = 'loop';

/// Default sampler play mode (Tauri `DEFAULT_SAMPLER_PLAY_MODE`).
const String kDefaultSamplerPlayMode = kSamplerPlayModeOneshot;

/// Bank settings dialog options (`default` = inherit settings).
const kSamplerPlayModeOptions = <String>[
  'default',
  kSamplerPlayModeOneshot,
  kSamplerPlayModeHold,
  kSamplerPlayModeLoop,
];

/// Stem names for pad slots 0–3 (mute) and 4–7 (isolate/solo).
const kStemNames = <String>['drums', 'bass', 'other', 'vocals'];

String padModeShortLabel(PadMode mode) => switch (mode) {
  PadMode.hotCue => 'Cue',
  PadMode.loopRoll => 'Roll',
  PadMode.beatJump => 'Jump',
  PadMode.sampler => 'Sample',
  PadMode.stems => 'Stems',
  PadMode.keyboard => 'Keys',
  PadMode.keyShift => 'Shift',
};

String keyboardScaleShortLabel(KeyboardScale scale) => switch (scale) {
  KeyboardScale.major => 'Major',
  KeyboardScale.minor => 'Minor',
  KeyboardScale.pentatonic => 'Penta',
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
