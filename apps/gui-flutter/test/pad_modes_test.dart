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

  test('key shift pad semitones and scale degrees match engine', () {
    expect(kKeyShiftPadSemitones, [0, 1, 2, 3, -4, -3, -2, -1]);
    expect(kKeyboardScaleDegrees[KeyboardScale.major], [
      0,
      2,
      4,
      5,
      7,
      9,
      11,
      12,
    ]);
    expect(kKeyboardScaleDegrees[KeyboardScale.minor], [
      0,
      2,
      3,
      5,
      7,
      8,
      10,
      12,
    ]);
    expect(kKeyboardScaleDegrees[KeyboardScale.pentatonic], [
      0,
      2,
      4,
      7,
      9,
      12,
      14,
      16,
    ]);
    expect(keyboardScaleShortLabel(KeyboardScale.major), 'Major');
    expect(keyboardScaleShortLabel(KeyboardScale.minor), 'Minor');
    expect(keyboardScaleShortLabel(KeyboardScale.pentatonic), 'Penta');
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
