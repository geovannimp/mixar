import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/mixer/deck_loop_panel.dart';

void main() {
  test('kAutoLoopBeats matches Tauri list', () {
    expect(kAutoLoopBeats, [1, 2, 4, 8, 16, 32]);
  });

  test('autoLoopBeatIndex falls back to 4', () {
    expect(autoLoopBeatIndex(4), 2);
    expect(autoLoopBeatIndex(99), 2);
  });

  test('stepAutoLoopBeats clamps at ends', () {
    expect(stepAutoLoopBeats(1, -1), 1);
    expect(stepAutoLoopBeats(1, 1), 2);
    expect(stepAutoLoopBeats(32, 1), 32);
    expect(stepAutoLoopBeats(32, -1), 16);
  });
}
