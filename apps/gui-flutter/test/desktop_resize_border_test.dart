import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/utils/desktop.dart';
import 'package:gui_flutter/widgets/shell/desktop_resize_border.dart';
import 'package:window_manager/window_manager.dart';

const _child = SizedBox(width: 120, height: 80);

Future<void> _pumpBorder(WidgetTester tester) async {
  await tester.pumpWidget(
    const Directionality(
      textDirection: TextDirection.ltr,
      child: DesktopResizeBorder(child: _child),
    ),
  );
}

Future<void> _asHost({
  required bool desktop,
  required Future<void> Function() body,
  bool? maximized,
}) async {
  debugOverrideDesktopWindow = desktop;
  debugOverrideWindowMaximized = maximized;
  try {
    await body();
  } finally {
    debugOverrideDesktopWindow = null;
    debugOverrideWindowMaximized = null;
  }
}

void main() {
  testWidgets('desktop windows get edge resize handles', (tester) async {
    await _asHost(
      desktop: true,
      maximized: false,
      body: () async {
        await _pumpBorder(tester);
        final area = tester.widget<DragToResizeArea>(
          find.byType(DragToResizeArea),
        );
        expect(area.resizeEdgeSize, 8);
        expect(area.enableResizeEdges, isNull, reason: 'all edges enabled');
        expect(find.byWidget(_child), findsOneWidget);
      },
    );
  });

  testWidgets('maximized windows disable the handles', (tester) async {
    await _asHost(
      desktop: true,
      maximized: true,
      body: () async {
        await _pumpBorder(tester);
        final area = tester.widget<DragToResizeArea>(
          find.byType(DragToResizeArea),
        );
        expect(area.enableResizeEdges, isEmpty);
      },
    );
  });

  testWidgets('non-desktop hosts keep their child untouched', (tester) async {
    await _asHost(
      desktop: false,
      body: () async {
        await _pumpBorder(tester);
        expect(find.byType(DragToResizeArea), findsNothing);
        expect(find.byWidget(_child), findsOneWidget);
      },
    );
  });
}
