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
      var kills = 0;
      var closes = 0;
      var exits = 0;
      runCloseSequence(
        linux: true,
        forceKill: () {
          kills++;
          return true;
        },
        closeWindow: () => closes++,
        exitApp: (_) => exits++,
      );
      expect(kills, 1);
      expect(closes, 0, reason: 'a blocking close would outlive the kill');
      expect(exits, 0);
    });

    test('a kill that did not land falls through to close and exit', () {
      var closes = 0;
      var exits = 0;
      runCloseSequence(
        linux: true,
        forceKill: () => false,
        closeWindow: () => closes++,
        exitApp: (_) => exits++,
      );
      expect(closes, 1);
      expect(exits, 1);
    });

    test('other hosts never try to kill', () {
      var kills = 0;
      var closes = 0;
      var exits = 0;
      runCloseSequence(
        linux: false,
        forceKill: () {
          kills++;
          return true;
        },
        closeWindow: () => closes++,
        exitApp: (_) => exits++,
      );
      expect(kills, 0);
      expect(closes, 1);
      expect(exits, 1);
    });
  });
}
