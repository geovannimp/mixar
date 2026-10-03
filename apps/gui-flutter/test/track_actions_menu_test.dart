import 'package:flutter/gestures.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/library/providers.dart';
import 'package:gui_flutter/library/track_list.dart';
import 'package:gui_flutter/library/track_list_pane.dart';
import 'package:gui_flutter/mixer/engine_providers.dart';
import 'package:gui_flutter/mixer/engine_ui.dart';
import 'package:gui_flutter/mixer/track_drag.dart';
import 'package:gui_flutter/shell/m_divider.dart';
import 'package:gui_flutter/shell/material_theme.dart';
import 'package:gui_flutter/shell/mixar_menu.dart';
import 'package:gui_flutter/shell/mixar_theme.dart';
import 'package:gui_flutter/src/rust/api/library.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';
import 'package:material_ui/material_ui.dart';

import 'support/mixar_material_app.dart';

class _RunningEngineUi extends EngineUi {
  @override
  EngineUiSnapshot build() =>
      const EngineUiSnapshot(running: true, trackPaths: {});
}

MixarMenuSegment _loadSegment(WidgetTester tester, String letter) {
  final segments = tester
      .widget<MixarMenuSegments>(find.byType(MixarMenuSegments))
      .segments;
  return segments.firstWhere(
    (segment) => segment.semanticsLabel == 'Load to $letter',
  );
}

void main() {
  const track = LibraryTrackSummary(
    id: 't1',
    displayName: 'Track',
    title: 'Track',
    path: '/tmp/t1.wav',
  );
  const payload = TrackDragPayload(
    source: TrackDragSource.library,
    trackId: 't1',
    path: '/tmp/t1.wav',
    title: 'Track',
  );

  Future<ProviderContainer> pumpMenu(
    WidgetTester tester, {
    bool running = false,
    bool stemsGenerating = false,
    double width = 200,
  }) async {
    final theme = MixarThemeData.dark();
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          driveResolvedByPathProvider.overrideWith(
            (ref) async => const <String, LibraryTrackSummary>{},
          ),
          if (running) engineUiProvider.overrideWith(_RunningEngineUi.new),
        ],
        child: MaterialApp(
          theme: materialUiThemeFromMixar(theme),
          builder: mixarMaterialAppBuilder(theme),
          home: Scaffold(
            body: SizedBox(
              width: width,
              height: 36,
              child: TrackListMenuButton(
                // Match the production row: the 3-dot only owns the primary
                // toggle; secondary press is delegated to the row's context
                // menu (exercised in track_list_pane_test).
                enableSecondaryPress: false,
                menuBuilder: (context, ref, dismiss) => buildTrackListMenuBody(
                  context: context,
                  ref: ref,
                  payload: payload,
                  // The real track items, so the menu under test is the one the
                  // library list renders.
                  extras: trackRowExtraMenuItems(context, ref, track, dismiss),
                  dismiss: dismiss,
                ),
              ),
            ),
          ),
        ),
      ),
    );
    final container = ProviderScope.containerOf(
      tester.element(find.byType(TrackListMenuButton)),
    );
    if (stemsGenerating) {
      container
          .read(stemGeneratingTrackIdsProvider.notifier)
          .setGenerating(track.id, true);
      await tester.pump();
    }
    return container;
  }

  testWidgets('⋯ icon fits a 40px table cell', (tester) async {
    Object? overflow;
    final previous = FlutterError.onError;
    FlutterError.onError = (details) {
      if (details.toString().contains('overflowed')) {
        overflow = details.exception;
      }
      previous?.call(details);
    };
    addTearDown(() => FlutterError.onError = previous);

    await pumpMenu(tester, width: 40);
    expect(overflow, isNull);
    expect(find.byIcon(LucideIcons.ellipsisVertical), findsOneWidget);
  });

  testWidgets('Analyze is enabled for library tracks', (tester) async {
    await pumpMenu(tester);
    await tester.tap(find.byIcon(LucideIcons.ellipsisVertical));
    await tester.pumpAndSettle();
    expect(find.text('Analyze'), findsOneWidget);
    expect(tester.getSize(find.text('Load to deck')).width, lessThan(300));
  });

  testWidgets('the Load to deck header is not announced as a button', (
    tester,
  ) async {
    await pumpMenu(tester);
    await tester.tap(find.byIcon(LucideIcons.ellipsisVertical));
    await tester.pumpAndSettle();

    // A static group header must not carry button semantics or be skipped by
    // focus traversal the way a disabled MixarMenuItem would.
    final node = tester.getSemantics(
      find
          .byWidgetPredicate(
            (widget) => widget is Text && widget.data == 'Load to deck',
          )
          .first,
    );
    expect(node.flagsCollection.isButton, isFalse);
  });

  testWidgets('Generate stems is its own action for library tracks', (
    tester,
  ) async {
    await pumpMenu(tester);
    await tester.tap(find.byIcon(LucideIcons.ellipsisVertical));
    await tester.pumpAndSettle();
    // Stems no longer ride along with Analyze, so they get a separate item.
    expect(find.text('Generate stems'), findsOneWidget);
  });

  testWidgets('Generate stems reports progress and blocks a second run', (
    tester,
  ) async {
    await pumpMenu(tester, stemsGenerating: true);
    await tester.tap(find.byIcon(LucideIcons.ellipsisVertical));
    await tester.pumpAndSettle();
    expect(find.text('Generating stems…'), findsOneWidget);
    expect(find.text('Generate stems'), findsNothing);
  });

  testWidgets('Load to A/B is disabled when the engine is stopped', (
    tester,
  ) async {
    await pumpMenu(tester);
    await tester.tap(find.byIcon(LucideIcons.ellipsisVertical));
    await tester.pumpAndSettle();
    expect(find.text('Load to deck'), findsOneWidget);
    expect(_loadSegment(tester, 'A').onPress, isNull);
    expect(_loadSegment(tester, 'B').onPress, isNull);
  });

  testWidgets('Load to A/B is enabled when the engine is running', (
    tester,
  ) async {
    await pumpMenu(tester, running: true);
    await tester.tap(find.byIcon(LucideIcons.ellipsisVertical));
    await tester.pumpAndSettle();
    expect(find.text('Load to deck'), findsOneWidget);
    expect(_loadSegment(tester, 'A').onPress, isNotNull);
    expect(_loadSegment(tester, 'B').onPress, isNotNull);
  });

  test('collection tracks stay in-library when id equals path', () {
    const track = LibraryTrackSummary(
      id: '/music/a.wav',
      displayName: 'a.wav',
      path: '/music/a.wav',
    );
    expect(trackIsInLibrary(track, tab: LibrarySourceTab.collections), isTrue);
    expect(trackIsInLibrary(track, tab: LibrarySourceTab.drive), isFalse);
    expect(
      trackIsInLibrary(
        track,
        tab: LibrarySourceTab.drive,
        driveResolvedByPath: {track.path: track},
      ),
      isTrue,
    );
  });

  group('MixarMenuSegments', () {
    Future<void> pumpSegments(
      WidgetTester tester, {
      required List<MixarMenuSegment> segments,
    }) async {
      final theme = MixarThemeData.dark();
      await tester.pumpWidget(
        MaterialApp(
          theme: materialUiThemeFromMixar(theme),
          builder: mixarMaterialAppBuilder(theme),
          home: Scaffold(
            body: Center(
              child: SizedBox(
                width: 200,
                child: MixarMenuSegments(segments: segments),
              ),
            ),
          ),
        ),
      );
    }

    Color segmentPaint(WidgetTester tester, String label) {
      final box = tester.widget<ColoredBox>(
        find
            .ancestor(of: find.text(label), matching: find.byType(ColoredBox))
            .first,
      );
      return box.color;
    }

    testWidgets('a disabled segment does not highlight on hover', (
      tester,
    ) async {
      final theme = MixarThemeData.dark();
      await pumpSegments(
        tester,
        segments: const [
          MixarMenuSegment(label: 'A', semanticsLabel: 'Load to A'),
          MixarMenuSegment(
            label: 'B',
            semanticsLabel: 'Load to B',
            onPress: _noop,
          ),
        ],
      );

      final gesture = await tester.createGesture(kind: PointerDeviceKind.mouse);
      await gesture.addPointer(location: Offset.zero);
      addTearDown(gesture.removePointer);
      await tester.pump();

      await gesture.moveTo(tester.getCenter(find.text('A')));
      await tester.pumpAndSettle();

      expect(
        segmentPaint(tester, 'A'),
        isNot(theme.colors.selection),
        reason: 'a disabled segment must not paint the hover highlight',
      );
    });

    testWidgets('an enabled segment highlights on hover', (tester) async {
      final theme = MixarThemeData.dark();
      await pumpSegments(
        tester,
        segments: const [
          MixarMenuSegment(
            label: 'A',
            semanticsLabel: 'Load to A',
            onPress: _noop,
          ),
        ],
      );

      final gesture = await tester.createGesture(kind: PointerDeviceKind.mouse);
      await gesture.addPointer(location: Offset.zero);
      addTearDown(gesture.removePointer);
      await tester.pump();

      await gesture.moveTo(tester.getCenter(find.text('A')));
      await tester.pumpAndSettle();

      expect(segmentPaint(tester, 'A'), theme.colors.selection);
    });

    testWidgets('an empty segment list paints nothing', (tester) async {
      await pumpSegments(tester, segments: const []);
      expect(find.byType(MixarMenuSegments), findsOneWidget);
      expect(find.byType(MDivider), findsNothing);
    });

    test('a short label without a semanticsLabel is rejected', () {
      expect(
        () => MixarMenuSegment(label: 'A'),
        throwsA(isA<AssertionError>()),
      );
      // Long labels carry their own meaning, so they need no override.
      expect(() => const MixarMenuSegment(label: 'Archive'), returnsNormally);
    });
  });
}

void _noop() {}
