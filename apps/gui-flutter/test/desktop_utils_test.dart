import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/utils/desktop.dart';

void main() {
  group('currentNativeWindow', () {
    test('is null when the host has no desktop window', () {
      _withHost(
        desktop: false,
        body: () => expect(currentNativeWindow(), isNull),
      );
    });

    test('is null when a desktop host reports no window', () {
      // Widget tests have no native host: the lookup must answer null instead
      // of throwing, which is what every call site's guard relies on.
      _withHost(
        desktop: true,
        body: () => expect(currentNativeWindow(), isNull),
      );
    });
  });
}

/// Runs [body] with the desktop-window override set, restoring it after.
void _withHost({required bool desktop, required void Function() body}) {
  debugOverrideDesktopWindow = desktop;
  addTearDown(() => debugOverrideDesktopWindow = null);
  body();
}
