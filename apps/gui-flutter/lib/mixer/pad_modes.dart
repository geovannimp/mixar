import 'package:flutter/foundation.dart' show immutable;
import 'package:flutter/services.dart';
import 'package:gui_flutter/src/rust/api/engine.dart' show PadMode;

export 'package:gui_flutter/src/rust/api/engine.dart' show PadMode;

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

/// Number of Keyboard semitone pages (pitch-only; no utility page).
const kKeyboardPageCount = 4;

/// Number of Key Shift semitone pages (includes the utility page).
const kKeyShiftPageCount = 5;

/// Default Keyboard / Key Shift page (`[0..+7]`), matching the engine.
const kDefaultPitchPage = 2;

/// Action a Keyboard or Key Shift pad performs within a semitone page.
enum PitchPadAction {
  /// Absolute semitone offset from the root.
  semitone,

  /// Reset the deck key shift to `0`.
  keyReset,

  /// Nudge the deck key shift up one semitone.
  semitoneUp,

  /// Nudge the deck key shift down one semitone.
  semitoneDown,

  /// Key sync (not implemented → no-op).
  keySync,

  /// No action.
  none,
}

/// One Keyboard / Key Shift page slot: an absolute semitone, a page-5 special,
/// or an empty slot.
@immutable
class PitchPad {
  const new(this.action, [this.semitones = 0]);

  final PitchPadAction action;

  /// Absolute semitone offset; meaningful only when [action] is
  /// [PitchPadAction.semitone].
  final int semitones;

  @override
  bool operator ==(Object other) =>
      other is PitchPad &&
      action == other.action &&
      semitones == other.semitones;

  @override
  int get hashCode => Object.hash(action, semitones);
}

const _none = PitchPad(PitchPadAction.none);
const _reset = PitchPad(PitchPadAction.keyReset);
const _up = PitchPad(PitchPadAction.semitoneUp);
const _down = PitchPad(PitchPadAction.semitoneDown);
const _sync = PitchPad(PitchPadAction.keySync);

/// Rekordbox DDJ-400 footnote *6 Keyboard page table; slot 0 = pad 1 = root.
///
/// Keyboard is pitch-only: four pages, no Reset/Up/Down/Sync utility page.
const kKeyboardPages = <List<PitchPad>>[
  // PAGE 1 — "+8…+12"
  [
    PitchPad(PitchPadAction.semitone, 8),
    PitchPad(PitchPadAction.semitone, 9),
    PitchPad(PitchPadAction.semitone, 10),
    PitchPad(PitchPadAction.semitone, 11),
    PitchPad(PitchPadAction.semitone, 12),
    _none,
    _none,
    _none,
  ],
  // PAGE 2 (default) — "0…+7"
  [
    PitchPad(PitchPadAction.semitone),
    PitchPad(PitchPadAction.semitone, 1),
    PitchPad(PitchPadAction.semitone, 2),
    PitchPad(PitchPadAction.semitone, 3),
    PitchPad(PitchPadAction.semitone, 4),
    PitchPad(PitchPadAction.semitone, 5),
    PitchPad(PitchPadAction.semitone, 6),
    PitchPad(PitchPadAction.semitone, 7),
  ],
  // PAGE 3 — "-1…-8"
  [
    PitchPad(PitchPadAction.semitone, -8),
    PitchPad(PitchPadAction.semitone, -7),
    PitchPad(PitchPadAction.semitone, -6),
    PitchPad(PitchPadAction.semitone, -5),
    PitchPad(PitchPadAction.semitone, -4),
    PitchPad(PitchPadAction.semitone, -3),
    PitchPad(PitchPadAction.semitone, -2),
    PitchPad(PitchPadAction.semitone, -1),
  ],
  // PAGE 4 — "-9…-12"
  [
    _none,
    _none,
    _none,
    _none,
    PitchPad(PitchPadAction.semitone, -12),
    PitchPad(PitchPadAction.semitone, -11),
    PitchPad(PitchPadAction.semitone, -10),
    PitchPad(PitchPadAction.semitone, -9),
  ],
];

/// Rekordbox DDJ-400 footnote *6 Key Shift page table; slot 0 = pad 1 = root.
///
/// Key Shift shares pages 1–4 with Keyboard plus a utility page 5.
const kKeyShiftPages = <List<PitchPad>>[
  // PAGE 1 — "+8…+12"
  [
    PitchPad(PitchPadAction.semitone, 8),
    PitchPad(PitchPadAction.semitone, 9),
    PitchPad(PitchPadAction.semitone, 10),
    PitchPad(PitchPadAction.semitone, 11),
    PitchPad(PitchPadAction.semitone, 12),
    _none,
    _none,
    _none,
  ],
  // PAGE 2 (default) — "0…+7"
  [
    PitchPad(PitchPadAction.semitone),
    PitchPad(PitchPadAction.semitone, 1),
    PitchPad(PitchPadAction.semitone, 2),
    PitchPad(PitchPadAction.semitone, 3),
    PitchPad(PitchPadAction.semitone, 4),
    PitchPad(PitchPadAction.semitone, 5),
    PitchPad(PitchPadAction.semitone, 6),
    PitchPad(PitchPadAction.semitone, 7),
  ],
  // PAGE 3 — "-1…-8"
  [
    PitchPad(PitchPadAction.semitone, -8),
    PitchPad(PitchPadAction.semitone, -7),
    PitchPad(PitchPadAction.semitone, -6),
    PitchPad(PitchPadAction.semitone, -5),
    PitchPad(PitchPadAction.semitone, -4),
    PitchPad(PitchPadAction.semitone, -3),
    PitchPad(PitchPadAction.semitone, -2),
    PitchPad(PitchPadAction.semitone, -1),
  ],
  // PAGE 4 — "-9…-12"
  [
    _none,
    _none,
    _none,
    _none,
    PitchPad(PitchPadAction.semitone, -12),
    PitchPad(PitchPadAction.semitone, -11),
    PitchPad(PitchPadAction.semitone, -10),
    PitchPad(PitchPadAction.semitone, -9),
  ],
  // PAGE 5 (utility) — "UTIL"
  [
    _reset,
    _down,
    PitchPad(PitchPadAction.semitone, -5),
    PitchPad(PitchPadAction.semitone, -12),
    _sync,
    _up,
    PitchPad(PitchPadAction.semitone, 7),
    PitchPad(PitchPadAction.semitone, 12),
  ],
];

/// Keyboard page slots for [page] (`1..=4`), clamped to a valid page.
List<PitchPad> keyboardPage(int page) =>
    kKeyboardPages[(page.clamp(1, kKeyboardPageCount)) - 1];

/// Key Shift page slots for [page] (`1..=5`), clamped to a valid page.
List<PitchPad> keyShiftPage(int page) =>
    kKeyShiftPages[(page.clamp(1, kKeyShiftPageCount)) - 1];

/// Semitone range label for a Keyboard [page] (e.g. `0…+7`).
String keyboardPageRangeLabel(int page) =>
    switch (page.clamp(1, kKeyboardPageCount)) {
      1 => '+8…+12',
      2 => '0…+7',
      3 => '-1…-8',
      _ => '-9…-12',
    };

/// Semitone range label for a Key Shift [page]; page 5 is the utility page.
String keyShiftPageRangeLabel(int page) =>
    switch (page.clamp(1, kKeyShiftPageCount)) {
      1 => '+8…+12',
      2 => '0…+7',
      3 => '-1…-8',
      4 => '-9…-12',
      _ => 'UTIL',
    };

/// Pad label: `"+4"`, `"0"`, `"-12"`, `"RESET"`, `"UP"`, `"DOWN"`, `"SYNC"`.
String pitchPadLabel(PitchPad pad) => switch (pad.action) {
  PitchPadAction.semitone =>
    pad.semitones > 0 ? '+${pad.semitones}' : '${pad.semitones}',
  PitchPadAction.keyReset => 'RESET',
  PitchPadAction.semitoneUp => 'UP',
  PitchPadAction.semitoneDown => 'DOWN',
  PitchPadAction.keySync => 'SYNC',
  PitchPadAction.none => '',
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
