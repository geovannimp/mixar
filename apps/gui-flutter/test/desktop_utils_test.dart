import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/utils/desktop.dart';

void main() {
  group('currentNativeWindow', () {
    test('is null when the host has no desktop window', () {
      debugOverrideDesktopWindow = false;
      addTearDown(() => debugOverrideDesktopWindow = null);
      expect(currentNativeWindow(), isNull);
    });

    test('is null when a desktop host reports no window', () {
      // Widget tests have no native host: the lookup must answer null instead
      // of throwing, which is what every call site's guard relies on.
      debugOverrideDesktopWindow = true;
      addTearDown(() => debugOverrideDesktopWindow = null);
      expect(currentNativeWindow(), isNull);
    });
  });
}
