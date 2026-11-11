import 'package:flutter/foundation.dart'
    show TargetPlatform, debugDefaultTargetPlatformOverride;
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/utils/library_list_layout.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/core/utils/track_drag.dart';
import 'package:gui_flutter/providers/library/drawer_providers.dart';
import 'package:gui_flutter/providers/mixer/engine_providers.dart';
import 'package:gui_flutter/providers/settings/settings_providers.dart';
import 'package:gui_flutter/widgets/library/library_drag_card.dart';
import 'package:gui_flutter/widgets/library/track_list_row.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:material_ui/material_ui.dart';

import 'support/mixar_material_app.dart';
import 'support/row_drag.dart';

/// Runs [body] with [platform] as the target platform, restoring the debug
/// override before the framework's end-of-test invariant check.
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
  const payload = TrackDragPayload(
    source: TrackDragSource.library,
    trackId: 't1',
    path: '/tmp/t1.wav',
    title: 'Track',
  );

  double scrollOffset(WidgetTester tester) =>
      tester.state<ScrollableState>(find.byType(Scrollable)).position.pixels;

  /// The shared drag source in a scrolling list — the shape every library pane
  /// uses, which is where a scroll and a drag compete for the same pan.
  Future<void> pumpDragList(
    WidgetTester tester, {
    required VoidCallback onStart,
    required VoidCallback onEnd,
  }) async {
    await tester.pumpWidget(
      mixarTestMaterialApp(
        theme: MixarThemeData.light(),
        home: ListView.builder(
          itemExtent: 56,
          itemCount: 30,
          itemBuilder: (context, index) => libraryTrackDragSource(
            payload: payload,
            onDragStart: onStart,
            onDragEnd: onEnd,
            // A coloured box, not a bare `SizedBox`: an empty box is not hit
            // testable, so the drag source would never see the pointer.
            child: ColoredBox(
              color: const Color(0xFF202020),
              child: Center(child: Text('row $index')),
            ),
          ),
        ),
      ),
    );
  }

  group('drag source', () {
    testWidgets('a touch pan scrolls the list instead of dragging a row', (
      tester,
    ) async {
      await withPlatform(TargetPlatform.android, () async {
        var starts = 0;
        var ends = 0;
        await pumpDragList(
          tester,
          onStart: () => starts++,
          onEnd: () => ends++,
        );

        final gesture = await tester.startGesture(
          tester.getCenter(find.text('row 1')),
        );
        await gesture.moveBy(const Offset(0, -40));
        await tester.pump();
        await gesture.moveBy(const Offset(0, -60));
        await tester.pump();
        await gesture.up();
        await tester.pump();

        expect(starts, 0, reason: 'a finger scroll must never pick a track up');
        expect(ends, 0);
        expect(scrollOffset(tester), greaterThan(0));
      });
    });

    testWidgets('a long press arms the drag and the next pan takes it', (
      tester,
    ) async {
      await withPlatform(TargetPlatform.android, () async {
        var starts = 0;
        var ends = 0;
        await pumpDragList(
          tester,
          onStart: () => starts++,
          onEnd: () => ends++,
        );

        final gesture = await tester.startGesture(
          tester.getCenter(find.text('row 1')),
        );
        await tester.pump(rowDragArm);
        await gesture.moveBy(const Offset(0, -30));
        await tester.pump();

        expect(starts, 1);
        expect(
          scrollOffset(tester),
          0,
          reason: 'the drag owns the pointer once the press lands',
        );
        expect(find.byType(LibraryDragCard), findsOneWidget);

        await gesture.up();
        await tester.pump();

        expect(ends, 1);
      });
    });

    testWidgets('a mouse host still drags straight off the press', (
      tester,
    ) async {
      await withPlatform(TargetPlatform.linux, () async {
        var starts = 0;
        await pumpDragList(tester, onStart: () => starts++, onEnd: () {});

        final gesture = await tester.startGesture(
          tester.getCenter(find.text('row 1')),
        );
        await gesture.moveBy(const Offset(0, -30));
        await tester.pump();

        expect(starts, 1, reason: 'no long press is needed with a mouse');
        expect(scrollOffset(tester), 0);

        await gesture.up();
        await tester.pump();
      });
    });
  });

  group('row long press', () {
    /// One row with a menu body that names itself, so a test can tell whether
    /// the press opened the menu.
    Future<ProviderContainer> pumpRow(
      WidgetTester tester, {
      required bool engineRunning,
    }) async {
      await tester.pumpWidget(
        ProviderScope(
          overrides: [
            appSettingsProvider.overrideWith(
              (ref) async => defaultAppSettings(),
            ),
          ],
          child: mixarTestMaterialApp(
            theme: MixarThemeData.light(),
            home: Scaffold(
              body: SizedBox(
                width: 640,
                height: 56,
                child: TrackListRow(
                  index: 0,
                  payloadOf: (ref) => payload,
                  menuBuilder: (context, ref, dismiss) =>
                      const Text('row menu'),
                  contentBuilder: (context, ref, actionsSlot) => Row(
                    children: [
                      const Expanded(child: Text('Demo Track')),
                      actionsSlot,
                    ],
                  ),
                  onSelect: () {},
                ),
              ),
            ),
          ),
        ),
      );
      final container = ProviderScope.containerOf(
        tester.element(find.byType(TrackListRow)),
      );
      if (engineRunning) {
        container.read(engineUiProvider.notifier).setRunning(true);
        await tester.pumpAndSettle();
      }
      return container;
    }

    testWidgets('a touch host gives the press to the drag, not the menu', (
      tester,
    ) async {
      await withPlatform(TargetPlatform.android, () async {
        final container = await pumpRow(tester, engineRunning: true);

        final gesture = await tester.startGesture(
          tester.getCenter(find.text('Demo Track')),
        );
        await tester.pump(rowDragArm);
        await tester.pumpAndSettle();

        expect(
          find.text('row menu'),
          findsNothing,
          reason: 'the press arms the drag, so it cannot also open the menu',
        );
        expect(
          container.read(libraryTrackDragActiveProvider),
          isTrue,
          reason: 'the press itself picks the row up',
        );

        await gesture.moveBy(const Offset(0, -30));
        await tester.pump();
        expect(find.byType(LibraryDragCard), findsOneWidget);

        await gesture.up();
        await tester.pump();
      });
    });

    testWidgets('a mouse host keeps the long-press menu on a draggable row', (
      tester,
    ) async {
      await withPlatform(TargetPlatform.linux, () async {
        await pumpRow(tester, engineRunning: true);

        final gesture = await tester.startGesture(
          tester.getCenter(find.text('Demo Track')),
        );
        await tester.pump(rowDragArm);
        await tester.pumpAndSettle();

        expect(find.text('row menu'), findsOneWidget);

        await gesture.up();
        await tester.pumpAndSettle();
      });
    });

    testWidgets('a row with no drag source keeps its long-press menu', (
      tester,
    ) async {
      await withPlatform(TargetPlatform.android, () async {
        await pumpRow(tester, engineRunning: false);

        final gesture = await tester.startGesture(
          tester.getCenter(find.text('Demo Track')),
        );
        await tester.pump(rowDragArm);
        await tester.pumpAndSettle();

        expect(
          find.text('row menu'),
          findsOneWidget,
          reason: 'nothing else claims the press, so the menu still does',
        );

        await gesture.up();
        await tester.pumpAndSettle();
      });
    });
  });
}
