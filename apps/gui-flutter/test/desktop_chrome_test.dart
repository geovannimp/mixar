import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/utils/desktop.dart';
import 'package:gui_flutter/widgets/shell/desktop_chrome.dart';

/// Runs [body] with [platform] as the target platform, restoring the debug
/// overrides before the framework's end-of-test invariant check.
Future<void> withPlatform(
  TargetPlatform platform,
  Future<void> Function() body,
) async {
  debugDefaultTargetPlatformOverride = platform;
  try {
    await body();
  } finally {
    debugDefaultTargetPlatformOverride = null;
  }
}

void main() {
  test('rounded window chrome is Linux only', () async {
    // Windows draws square window corners: the rounded clip shipped in
    // 0.1.0/0.1.1 left corner wedges against the desktop there.
    await withPlatform(TargetPlatform.windows, () async {
      expect(isRoundedDesktopChrome, isFalse);
    });
    await withPlatform(TargetPlatform.macOS, () async {
      expect(isRoundedDesktopChrome, isFalse);
    });
    await withPlatform(TargetPlatform.linux, () async {
      expect(isRoundedDesktopChrome, isTrue);
    });
  });

  testWidgets('a Windows host is not clipped to rounded corners', (
    tester,
  ) async {
    debugDefaultTargetPlatformOverride = TargetPlatform.windows;
    debugOverrideDesktopWindow = true;
    try {
      await tester.pumpWidget(
        const DesktopChrome(child: ColoredBox(color: Color(0xFF102030))),
      );
      expect(find.byType(ClipRRect), findsNothing);
    } finally {
      debugDefaultTargetPlatformOverride = null;
      debugOverrideDesktopWindow = null;
    }
  });
}
