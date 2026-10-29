import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/utils/desktop.dart';
import 'package:gui_flutter/widgets/shell/desktop_chrome.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';

import 'support/mixar_material_app.dart';

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

const _windowManagerChannel = MethodChannel('window_manager');

/// The window frame's stroke, if the chrome painted one.
Finder get _windowFrame => find.byWidgetPredicate(
  (widget) =>
      widget is DecoratedBox &&
      widget.position == DecorationPosition.foreground &&
      widget.decoration is BoxDecoration &&
      (widget.decoration as BoxDecoration).border != null,
);

/// Runs [body] as a desktop Linux host whose window reports [maximized].
///
/// [DesktopChrome] queries window_manager, which has no host in widget tests.
Future<void> withWindowHost(
  WidgetTester tester, {
  required bool maximized,
  required Future<void> Function() body,
}) async {
  debugDefaultTargetPlatformOverride = TargetPlatform.linux;
  debugOverrideDesktopWindow = true;
  final messenger =
      TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
        ..setMockMethodCallHandler(
          _windowManagerChannel,
          (call) async => call.method == 'isMaximized' ? maximized : null,
        );
  try {
    await body();
  } finally {
    messenger.setMockMethodCallHandler(_windowManagerChannel, null);
    debugDefaultTargetPlatformOverride = null;
    debugOverrideDesktopWindow = null;
  }
}

/// Pumps the chrome; [theme] `null` mounts it outside any [MixarTheme].
Future<void> pumpChrome(WidgetTester tester, {MixarThemeData? theme}) async {
  const chrome = DesktopChrome(child: ColoredBox(color: Color(0xFF102030)));
  await tester.pumpWidget(
    theme == null
        ? const MaterialApp(home: chrome)
        : mixarTestMaterialApp(theme: theme, home: chrome),
  );
  await tester.pumpAndSettle();
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
      expect(_windowFrame, findsNothing);
    } finally {
      debugDefaultTargetPlatformOverride = null;
      debugOverrideDesktopWindow = null;
    }
  });

  for (final theme in [MixarThemeData.dark(), MixarThemeData.light()]) {
    testWidgets('the window frame is the section hairline '
        '(${theme.colors.brightness.name})', (tester) async {
      await withWindowHost(
        tester,
        maximized: false,
        body: () async {
          await pumpChrome(tester, theme: theme);
          // Same colour and width as MCard, MDivider and every pane split,
          // so the window edge reads as one more section border.
          final frame = tester.widget<DecoratedBox>(_windowFrame);
          final decoration = frame.decoration as BoxDecoration;
          expect(
            decoration.border,
            Border.all(
              color: theme.colors.border,
              width: theme.style.borderWidth,
            ),
          );
          // The stroke follows the rounded top corners.
          expect(
            decoration.borderRadius,
            const BorderRadius.only(
              topLeft: Radius.circular(kDesktopWindowRadius),
              topRight: Radius.circular(kDesktopWindowRadius),
            ),
          );
        },
      );
    });
  }

  testWidgets('a maximized window drops the frame', (tester) async {
    await withWindowHost(
      tester,
      maximized: true,
      body: () async {
        await pumpChrome(tester, theme: MixarThemeData.dark());
        expect(_windowFrame, findsNothing);
      },
    );
  });

  testWidgets('without a MixarTheme it clips but paints no frame', (
    tester,
  ) async {
    // A host that mounts the chrome outside the app's tree has no tokens to
    // stroke with; it must degrade, not fail on the missing ancestor.
    await withWindowHost(
      tester,
      maximized: false,
      body: () async {
        await pumpChrome(tester);
        expect(tester.takeException(), isNull);
        expect(find.byType(ClipRRect), findsOneWidget);
        expect(_windowFrame, findsNothing);
      },
    );
  });
}
