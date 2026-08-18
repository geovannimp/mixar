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
    ]);
    expect(padModeShortLabel(PadMode.hotCue), 'Cue');
    expect(padModeShortLabel(PadMode.loopRoll), 'Roll');
    expect(padModeShortLabel(PadMode.beatJump), 'Jump');
    expect(padModeShortLabel(PadMode.sampler), 'Sample');
  });

  test('cyclePadMode wraps', () {
    expect(cyclePadMode(PadMode.hotCue, 1), PadMode.loopRoll);
    expect(cyclePadMode(PadMode.sampler, 1), PadMode.hotCue);
    expect(cyclePadMode(PadMode.hotCue, -1), PadMode.sampler);
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
}
