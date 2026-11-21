import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/widgets/shell/window_actions.dart';

void main() {
  test(
    'closeApp leaves the host alone without a native window',
    // Widget tests (and any headless host) have no window: the helper must
    // return instead of ending the test process.
    closeApp,
  );

  group('runCloseSequence', () {
    test('a delivered SIGKILL ends the process before close and exit', () {
      final calls = _record(linux: true, killLands: true);
      expect(calls.kills, 1);
      expect(
        calls.closes,
        0,
        reason: 'a blocking close would outlive the kill',
      );
      expect(calls.exits, 0);
    });

    test('a kill that did not land falls through to close and exit', () {
      final calls = _record(linux: true, killLands: false);
      expect((calls.kills, calls.closes, calls.exits), (1, 1, 1));
    });

    test('other hosts never try to kill', () {
      final calls = _record(linux: false, killLands: true);
      expect((calls.kills, calls.closes, calls.exits), (0, 1, 1));
    });
  });
}

/// Runs the sequence against recording callbacks and reports what each counted.
({int kills, int closes, int exits}) _record({
  required bool linux,
  required bool killLands,
}) {
  var kills = 0;
  var closes = 0;
  var exits = 0;
  runCloseSequence(
    linux: linux,
    forceKill: () {
      kills++;
      return killLands;
    },
    closeWindow: () => closes++,
    exitApp: (_) => exits++,
  );
  return (kills: kills, closes: closes, exits: exits);
}
