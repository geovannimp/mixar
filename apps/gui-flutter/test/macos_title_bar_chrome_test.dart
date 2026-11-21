import 'package:flutter/foundation.dart';
import 'package:flutter/gestures.dart';
import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/utils/desktop.dart';
import 'package:gui_flutter/widgets/shell/m_tappable.dart';
import 'package:gui_flutter/widgets/shell/macos_title_bar_chrome.dart';
import 'package:window_manager/window_manager.dart';

/// The marks a hovered cluster paints; the painter is private, so the glyph
/// keys are the handle.
const _closeGlyph = Key('macos-close-glyph');
const _minimizeGlyph = Key('macos-minimize-glyph');
const _zoomGlyph = Key('macos-zoom-glyph');

/// The preview's own label. The real macOS cluster must never carry it.
const _previewLabel = 'macOS title-bar layout preview';

Finder get _previewMarkers => find.byWidgetPredicate(
  (widget) => widget is Semantics && widget.properties.label == _previewLabel,
);

/// A light: the header child has no round decorations of its own.
Finder get _lights => find.descendant(
  of: find.byType(MacOSTitleBarChrome),
  matching: find.byWidgetPredicate(
    (widget) =>
        widget is DecoratedBox &&
        widget.decoration is BoxDecoration &&
        (widget.decoration as BoxDecoration).shape == BoxShape.circle,
  ),
);

/// The interactive light with [label], on the macOS cluster.
Finder _light(String label) => find.byWidgetPredicate(
  (widget) => widget is MTappable && widget.semanticsLabel == label,
);

/// Runs [body] as a [platform] host with [desktop] and the macOS chrome
/// preview [preview], restoring every override afterwards.
Future<void> withHost(
  TargetPlatform platform, {
  required bool desktop,
  required bool? preview,
  required Future<void> Function() body,
}) async {
  debugDefaultTargetPlatformOverride = platform;
  debugOverrideDesktopWindow = desktop;
  debugOverrideMacOSChromePreview = preview;
  try {
    await body();
  } finally {
    debugDefaultTargetPlatformOverride = null;
    debugOverrideDesktopWindow = null;
    debugOverrideMacOSChromePreview = null;
  }
}

const _childSize = Size(320, 40);

/// Pumps a header-sized child through the chrome, pinned top left.
///
/// [revision] keys the wrapper so a second pump actually rebuilds it: an
/// identical `const` tree would otherwise be left untouched.
Future<void> pumpChrome(WidgetTester tester, {int revision = 0}) async {
  await tester.pumpWidget(
    Directionality(
      textDirection: TextDirection.ltr,
      child: Align(
        alignment: Alignment.topLeft,
        child: MacOSTitleBarChrome(
          key: ValueKey(revision),
          child: const SizedBox(
            width: 320,
            height: 40,
            child: ColoredBox(color: Color(0xFF102030)),
          ),
        ),
      ),
    ),
  );
}

/// Fill of the light at [index], in tree order.
Color _dotColor(WidgetTester tester, int index) =>
    ((_lights.at(index).evaluate().single.widget as DecoratedBox).decoration
            as BoxDecoration)
        .color!;

/// The cluster sits in the middle of the 80px zone on both axes.
void expectCentredLights(WidgetTester tester) {
  expect(
    tester.getRect(find.byType(Row).first),
    const Rect.fromLTWH(11, 13, 58, 14),
    reason: '58px of lights centred in an 80px by 40px zone',
  );
  expect(
    [for (var i = 0; i < 3; i++) tester.getTopLeft(_lights.at(i)).dx],
    [11.0, 33.0, 55.0],
  );
  expect(_lights, findsNWidgets(3));
  expect(
    [for (var i = 0; i < 3; i++) tester.getSize(_lights.at(i))],
    const [Size.square(14), Size.square(14), Size.square(14)],
  );
}

void main() {
  setUp(() {
    debugOverrideDesktopWindow = null;
    debugOverrideMacOSChromePreview = null;
    debugDefaultTargetPlatformOverride = null;
  });
  tearDown(() {
    debugOverrideDesktopWindow = null;
    debugOverrideMacOSChromePreview = null;
    debugOverrideWindowFocused = null;
    debugDefaultTargetPlatformOverride = null;
  });

  testWidgets('off the macOS layouts the wrapper is the header itself', (
    tester,
  ) async {
    await withHost(
      TargetPlatform.linux,
      desktop: true,
      preview: false,
      body: () async {
        await pumpChrome(tester);

        expect(tester.getSize(find.byType(MacOSTitleBarChrome)), _childSize);
        expect(_lights, findsNothing, reason: 'no reference graphics on Linux');
        expect(_previewMarkers, findsNothing);
        expect(find.byType(Stack), findsNothing);
      },
    );
  });

  testWidgets('a macOS window draws the cluster as its own controls', (
    tester,
  ) async {
    await withHost(
      TargetPlatform.macOS,
      desktop: true,
      preview: null,
      body: () async {
        await pumpChrome(tester);

        expect(usesMacOSTitleBarLayout, isTrue);
        expect(
          isMacOSChromePreview,
          isFalse,
          reason: 'a Mac never shows the preview markers',
        );
        expect(
          tester.getSize(find.byType(MacOSTitleBarChrome)),
          _childSize,
          reason: 'the lights paint over the header, they do not size it',
        );
        expectCentredLights(tester);
        expect(_previewMarkers, findsNothing);

        // Real controls, not reference graphics.
        expect(_light('Close'), findsOneWidget);
        expect(_light('Minimize'), findsOneWidget);
        expect(_light('Zoom'), findsOneWidget);
        expect(
          find.byType(IgnorePointer),
          findsNothing,
          reason: 'the macOS cluster is the window’s only close button',
        );
      },
    );
  });

  testWidgets('an unfocused window greys the lights out', (tester) async {
    debugOverrideWindowFocused = false;
    await withHost(
      TargetPlatform.macOS,
      desktop: true,
      preview: null,
      body: () async {
        await pumpChrome(tester);
        expect(
          {for (var i = 0; i < 3; i++) _dotColor(tester, i)},
          {const Color(0xFF4C4E51)},
          reason: 'macOS greys every light while the window is inactive',
        );
        expect(
          _light('Close'),
          findsOneWidget,
          reason: 'the controls still work; they only read as inactive',
        );
        expect(find.byKey(_closeGlyph), findsNothing);
      },
    );
  });

  testWidgets('hovering the cluster reveals the glyphs', (tester) async {
    await withHost(
      TargetPlatform.macOS,
      desktop: true,
      preview: null,
      body: () async {
        await pumpChrome(tester);
        await tester.pump();
        expect(find.byKey(_closeGlyph), findsNothing);

        final mouse = await tester.createGesture(kind: PointerDeviceKind.mouse);
        await mouse.addPointer(location: Offset.zero);
        addTearDown(mouse.removePointer);

        await mouse.moveTo(tester.getCenter(find.byType(Row).first));
        await tester.pumpAndSettle();

        expect(find.byKey(_closeGlyph), findsOneWidget);
        expect(find.byKey(_minimizeGlyph), findsOneWidget);
        expect(find.byKey(_zoomGlyph), findsOneWidget);

        await mouse.moveTo(const Offset(400, 400));
        await tester.pumpAndSettle();
        expect(
          find.byKey(_closeGlyph),
          findsNothing,
          reason: 'the glyphs go away with the pointer',
        );
      },
    );
  });

  testWidgets("the preview lights drive the window like a Mac's do", (
    tester,
  ) async {
    await withHost(
      TargetPlatform.linux,
      desktop: true,
      preview: true,
      body: () async {
        await pumpChrome(tester);

        expect(isMacOSChromePreview, isTrue);
        expect(tester.getSize(find.byType(MacOSTitleBarChrome)), _childSize);
        expectCentredLights(tester);

        expect(_previewMarkers, findsOneWidget);
        expect(
          [
            for (var i = 0; i < 3; i++)
              ((_lights.at(i).evaluate().single.widget as DecoratedBox)
                          .decoration
                      as BoxDecoration)
                  .color,
          ],
          const [Color(0xFFFF5F57), Color(0xFFFEBC2E), Color(0xFF28C840)],
          reason: 'macOS close/minimise/zoom, not theme tokens',
        );

        // The same controls as on a Mac: the preview is where they get tried.
        expect(_light('Close'), findsOneWidget);
        expect(_light('Minimize'), findsOneWidget);
        expect(_light('Zoom'), findsOneWidget);

        await tester.tap(_light('Close'));
        await tester.pump();

        await tester.tap(_light('Zoom'));
        await tester.pumpAndSettle();
        expect(tester.takeException(), isNull);
      },
    );
  });

  test('the window setup runs from Dart everywhere except Linux', () async {
    await withHost(
      TargetPlatform.linux,
      desktop: true,
      preview: false,
      body: () async {
        expect(
          configuresWindowFromDart,
          isFalse,
          reason: 'the GTK runner sizes, centres and undecorates the window',
        );
      },
    );
    for (final platform in [TargetPlatform.macOS, TargetPlatform.windows]) {
      await withHost(
        platform,
        desktop: true,
        preview: false,
        body: () async {
          expect(configuresWindowFromDart, isTrue, reason: platform.name);
        },
      );
    }
  });

  test(
    'macOS hides AppKit’s buttons through the shared window setup',
    () async {
      for (final platform in [TargetPlatform.linux, TargetPlatform.windows]) {
        await withHost(
          platform,
          desktop: true,
          preview: false,
          body: () async {
            final window = _RecordingWindow();
            configureDesktopWindow(window, title: 'Mixar');
            expect(
              window.calls,
              contains('isWindowControlButtonsVisible=true'),
              reason: '${platform.name} keeps the native buttons',
            );
          },
        );
      }
      await withHost(
        TargetPlatform.macOS,
        desktop: true,
        preview: false,
        body: () async {
          final window = _RecordingWindow();
          configureDesktopWindow(window, title: 'Mixar');
          expect(
            window.calls,
            contains('isWindowControlButtonsVisible=false'),
            reason: 'the header draws them, so AppKit must not',
          );
          expect(
            window.calls.indexOf('titleBarStyle=hidden'),
            lessThan(
              window.calls.indexOf('isWindowControlButtonsVisible=false'),
            ),
            reason: 'the style re-shows the native buttons, so it goes first',
          );
        },
      );
      await withHost(
        TargetPlatform.macOS,
        desktop: false,
        preview: null,
        body: () async {
          final window = _RecordingWindow();
          configureDesktopWindow(window, title: 'Mixar');
          expect(
            window.calls,
            contains('isWindowControlButtonsVisible=true'),
            reason: 'a non-desktop Mac draws no chrome — keep the native ones',
          );
        },
      );
    },
  );

  testWidgets('the preview override cannot turn a Mac into a preview', (
    tester,
  ) async {
    await withHost(
      TargetPlatform.macOS,
      desktop: true,
      preview: true,
      body: () async {
        await pumpChrome(tester);

        expect(isMacOSChromePreview, isFalse);
        expect(_previewMarkers, findsNothing);
        expect(_light('Close'), findsOneWidget);
      },
    );
  });

  testWidgets('turning the preview off removes the markers', (tester) async {
    await withHost(
      TargetPlatform.linux,
      desktop: true,
      preview: true,
      body: () async {
        await pumpChrome(tester);
        expect(_previewMarkers, findsOneWidget);

        debugOverrideMacOSChromePreview = false;
        await pumpChrome(tester, revision: 1);

        expect(
          _previewMarkers,
          findsNothing,
          reason: 'the flag is read per build, not captured once',
        );
        expect(_lights, findsNothing);
        expect(tester.getSize(find.byType(MacOSTitleBarChrome)), _childSize);
      },
    );
  });
}

/// Records the calls [configureDesktopWindow] makes instead of reaching a host.
///
/// `Window` is an FFI handle with no test double, so the setup is asserted
/// through the calls it makes: the wiring that a getter assertion misses.
final class _RecordingWindow implements Window {
  final calls = <String>[];

  @override
  dynamic noSuchMethod(Invocation invocation) {
    final name = RegExp(r'"([^"]+)"')
        .firstMatch(invocation.memberName.toString())
        ?.group(1)
        ?.replaceAll('=', '');
    calls.add(
      invocation.isSetter
          ? '$name=${invocation.positionalArguments.first}'
          : '$name()',
    );
    return null;
  }
}
