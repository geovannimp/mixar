import 'package:flutter/gestures.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/models/engine_ui.dart';
import 'package:gui_flutter/core/theme/material_theme.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/core/utils/track_drag.dart';
import 'package:gui_flutter/providers/library/providers.dart';
import 'package:gui_flutter/providers/mixer/engine_providers.dart';
import 'package:gui_flutter/providers/settings/settings_providers.dart';
import 'package:gui_flutter/src/rust/api/library.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';
import 'package:gui_flutter/widgets/library/track_list_pane.dart';
import 'package:gui_flutter/widgets/shell/m_divider.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';
import 'package:material_ui/material_ui.dart';

import 'package:gui_flutter/widgets/library/track_list_menu_button.dart';
import 'package:gui_flutter/widgets/library/track_list_view.dart';
import 'package:gui_flutter/widgets/shell/mixar_menu_segments.dart';

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
    SelectStyleSetting selectStyle = SelectStyleSetting.auto,
  }) async {
    final theme = MixarThemeData.dark();
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          driveResolvedByPathProvider.overrideWith(
            (ref) async => const <String, LibraryTrackSummary>{},
          ),
          appSettingsProvider.overrideWith(
            (ref) async =>
                copyAppSettings(defaultAppSettings(), selectStyle: selectStyle),
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

  testWidgets('mobile track actions menu shows Track actions title', (
    tester,
  ) async {
    await pumpMenu(tester, selectStyle: SelectStyleSetting.mobile);
    // Wait for appSettings override so selectStyleSettingProvider sees mobile.
    await tester.pumpAndSettle();
    await tester.tap(find.byIcon(LucideIcons.ellipsisVertical));
    await tester.pumpAndSettle();
    expect(find.text('Track actions'), findsOneWidget);
    expect(find.text('Load to deck'), findsOneWidget);
  });

  testWidgets('mobile Load to deck section spacing matches action cards', (
    tester,
  ) async {
    await pumpMenu(tester, selectStyle: SelectStyleSetting.mobile);
    await tester.pumpAndSettle();
    await tester.tap(find.byIcon(LucideIcons.ellipsisVertical));
    await tester.pumpAndSettle();

    final labelBottom = tester.getBottomLeft(find.text('Load to deck')).dy;
    final segmentTrack = find.descendant(
      of: find.byType(MixarMenuSegments),
      matching: find.byWidgetPredicate(
        (w) =>
            w is DecoratedBox &&
            w.decoration is BoxDecoration &&
            (w.decoration as BoxDecoration).border != null,
      ),
    );
    expect(segmentTrack, findsOneWidget);
    final segmentTop = tester.getTopLeft(segmentTrack).dy;
    final segmentBottom = tester.getBottomLeft(segmentTrack).dy;
    final segmentLeft = tester.getTopLeft(segmentTrack).dx;

    final analyzeCard = find
        .ancestor(
          of: find.text('Analyze'),
          matching: find.byWidgetPredicate(
            (w) =>
                w is DecoratedBox &&
                w.decoration is BoxDecoration &&
                (w.decoration as BoxDecoration).border != null,
          ),
        )
        .first;
    final analyzeTop = tester.getTopLeft(analyzeCard).dy;
    final analyzeLeft = tester.getTopLeft(analyzeCard).dx;
    final generateCard = find
        .ancestor(
          of: find.text('Generate stems'),
          matching: find.byWidgetPredicate(
            (w) =>
                w is DecoratedBox &&
                w.decoration is BoxDecoration &&
                (w.decoration as BoxDecoration).border != null,
          ),
        )
        .first;
    final analyzeBottom = tester.getBottomLeft(analyzeCard).dy;
    final generateTop = tester.getTopLeft(generateCard).dy;

    // Full-bleed like action cards (no extra 12px track inset).
    expect(segmentLeft, moreOrLessEquals(analyzeLeft, epsilon: 0.5));

    // Section label shares the dialog title's left edge (not the card text inset).
    final titleLeft = tester.getTopLeft(find.text('Track actions')).dx;
    final labelLeft = tester.getTopLeft(find.text('Load to deck')).dx;
    expect(labelLeft, moreOrLessEquals(titleLeft, epsilon: 0.5));

    // Label → A/B gap should be at least the inter-card gap (not cramped).
    final labelToSegment = segmentTop - labelBottom;
    final cardGap = generateTop - analyzeBottom;
    expect(labelToSegment, greaterThanOrEqualTo(cardGap - 0.5));

    // A/B → Analyze uses the same gap as Analyze → Generate stems.
    final segmentToAnalyze = analyzeTop - segmentBottom;
    expect(segmentToAnalyze, moreOrLessEquals(cardGap, epsilon: 0.5));
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
