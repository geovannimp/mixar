import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/mixer/pad_format.dart';
import 'package:gui_flutter/mixer/pad_modes.dart';

void main() {
  test('pad mode order and short labels match Tauri', () {
    expect(kPadModes, [
      PadMode.hotCue,
      PadMode.loopRoll,
      PadMode.beatJump,
      PadMode.sampler,
      PadMode.stems,
      PadMode.keyboard,
      PadMode.keyShift,
    ]);
    expect(padModeShortLabel(PadMode.hotCue), 'Cue');
    expect(padModeShortLabel(PadMode.loopRoll), 'Roll');
    expect(padModeShortLabel(PadMode.beatJump), 'Jump');
    expect(padModeShortLabel(PadMode.sampler), 'Sample');
    expect(padModeShortLabel(PadMode.stems), 'Stems');
    expect(padModeShortLabel(PadMode.keyboard), 'Keys');
    expect(padModeShortLabel(PadMode.keyShift), 'Shift');
  });

  test('cycle includes keyboard and key shift', () {
    expect(cyclePadMode(PadMode.stems, 1), PadMode.keyboard);
    expect(cyclePadMode(PadMode.keyboard, 1), PadMode.keyShift);
    expect(cyclePadMode(PadMode.keyShift, 1), PadMode.hotCue);
  });

  test('keyboard pages are pitch-only', () {
    expect(kKeyboardPageCount, 4);
    expect(kDefaultPitchPage, 2);
    expect(kKeyboardPages.length, 4);
    expect(
      kKeyboardPages
          .expand((page) => page)
          .every(
            (pad) =>
                pad.action == PitchPadAction.semitone ||
                pad.action == PitchPadAction.none,
          ),
      isTrue,
    );
    expect(kKeyboardPages, const [
      [
        PitchPad(PitchPadAction.semitone, 8),
        PitchPad(PitchPadAction.semitone, 9),
        PitchPad(PitchPadAction.semitone, 10),
        PitchPad(PitchPadAction.semitone, 11),
        PitchPad(PitchPadAction.semitone, 12),
        PitchPad(PitchPadAction.none),
        PitchPad(PitchPadAction.none),
        PitchPad(PitchPadAction.none),
      ],
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
      [
        PitchPad(PitchPadAction.none),
        PitchPad(PitchPadAction.none),
        PitchPad(PitchPadAction.none),
        PitchPad(PitchPadAction.none),
        PitchPad(PitchPadAction.semitone, -12),
        PitchPad(PitchPadAction.semitone, -11),
        PitchPad(PitchPadAction.semitone, -10),
        PitchPad(PitchPadAction.semitone, -9),
      ],
    ]);
  });

  test('key shift pages include the utility page', () {
    expect(kKeyShiftPageCount, 5);
    expect(kKeyShiftPages.length, 5);
    // Key Shift shares pages 1–4 with Keyboard.
    for (var page = 0; page < 4; page++) {
      expect(kKeyShiftPages[page], kKeyboardPages[page]);
    }
    expect(kKeyShiftPages[4], const [
      PitchPad(PitchPadAction.keyReset),
      PitchPad(PitchPadAction.semitoneDown),
      PitchPad(PitchPadAction.semitone, -5),
      PitchPad(PitchPadAction.semitone, -12),
      PitchPad(PitchPadAction.keySync),
      PitchPad(PitchPadAction.semitoneUp),
      PitchPad(PitchPadAction.semitone, 7),
      PitchPad(PitchPadAction.semitone, 12),
    ]);
  });

  test('page selectors clamp to each mode range', () {
    expect(keyboardPage(0), kKeyboardPages[0]);
    expect(keyboardPage(1), kKeyboardPages[0]);
    expect(keyboardPage(9), kKeyboardPages[3]);
    expect(keyboardPage(3), kKeyboardPages[2]);
    expect(keyShiftPage(0), kKeyShiftPages[0]);
    expect(keyShiftPage(6), kKeyShiftPages[4]);
    expect(keyShiftPage(3), kKeyShiftPages[2]);
  });

  test('page range labels match the semitone ranges', () {
    expect(keyboardPageRangeLabel(1), '+8…+12');
    expect(keyboardPageRangeLabel(2), '0…+7');
    expect(keyboardPageRangeLabel(3), '-1…-8');
    expect(keyboardPageRangeLabel(4), '-9…-12');
    // Keyboard clamps out-of-range pages to page 4 (never UTIL).
    expect(keyboardPageRangeLabel(5), '-9…-12');
    expect(keyShiftPageRangeLabel(1), '+8…+12');
    expect(keyShiftPageRangeLabel(2), '0…+7');
    expect(keyShiftPageRangeLabel(3), '-1…-8');
    expect(keyShiftPageRangeLabel(4), '-9…-12');
    expect(keyShiftPageRangeLabel(5), 'UTIL');
  });

  test('pitchPadLabel renders semitones and page-5 specials', () {
    expect(pitchPadLabel(const PitchPad(PitchPadAction.semitone, 4)), '+4');
    expect(pitchPadLabel(const PitchPad(PitchPadAction.semitone)), '0');
    expect(pitchPadLabel(const PitchPad(PitchPadAction.semitone, -12)), '-12');
    expect(pitchPadLabel(const PitchPad(PitchPadAction.keyReset)), 'RESET');
    expect(pitchPadLabel(const PitchPad(PitchPadAction.semitoneUp)), 'UP');
    expect(pitchPadLabel(const PitchPad(PitchPadAction.semitoneDown)), 'DOWN');
    expect(pitchPadLabel(const PitchPad(PitchPadAction.keySync)), 'SYNC');
    expect(pitchPadLabel(const PitchPad(PitchPadAction.none)), '');
  });

  test('cyclePadMode wraps', () {
    expect(cyclePadMode(PadMode.hotCue, 1), PadMode.loopRoll);
    expect(cyclePadMode(PadMode.sampler, 1), PadMode.stems);
    expect(cyclePadMode(PadMode.stems, 1), PadMode.keyboard);
    expect(cyclePadMode(PadMode.keyShift, 1), PadMode.hotCue);
    expect(cyclePadMode(PadMode.hotCue, -1), PadMode.keyShift);
  });

  test('beat tables match Tauri', () {
    expect(kLoopRollBeats, [1 / 32, 1 / 16, 1 / 8, 1 / 4, 1 / 2, 1, 2, 4]);
    expect(kBeatJumpForward, [1, 2, 4, 8, 16, 32, 64, 128]);
    expect(kBeatJumpBack, [-1, -2, -4, -8, -16, -32, -64, -128]);
  });

  test('formatBeatLength', () {
    expect(formatBeatLength(1 / 32), '1/32');
    expect(formatBeatLength(0.5), '1/2');
    expect(formatBeatLength(1), '1');
    expect(formatBeatLength(4), '4');
  });

  test('formatDeckTimeTenth', () {
    expect(formatDeckTimeTenth(null), '—');
    expect(formatDeckTimeTenth(-1), '—');
    expect(formatDeckTimeTenth(6500), '0:06.5');
    expect(formatDeckTimeTenth(125100), '2:05.1');
  });

  test('formatDeckRemainingDisplay and total', () {
    expect(formatDeckRemainingDisplay(null, 10000), '—');
    expect(formatDeckRemainingDisplay(2500, 10000), '-0:07.5');
    expect(formatDeckRemainingDisplay(12000, 10000), '-0:00.0');
    expect(formatDeckTotalDisplay(6500), '0:06.5');
  });
}
