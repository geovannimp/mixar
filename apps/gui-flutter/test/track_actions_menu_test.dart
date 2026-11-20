import 'dart:async';

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
import 'package:gui_flutter/widgets/library/track_list_menu_button.dart';
import 'package:gui_flutter/widgets/library/track_list_pane.dart';
import 'package:gui_flutter/widgets/library/track_list_view.dart';
import 'package:gui_flutter/widgets/shell/m_divider.dart';
import 'package:gui_flutter/widgets/shell/mixar_checkbox.dart';
import 'package:gui_flutter/widgets/shell/mixar_menu_item.dart';
import 'package:gui_flutter/widgets/shell/mixar_menu_segments.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';
import 'package:material_ui/material_ui.dart';

import 'support/fake_library_transport.dart';
import 'support/mixar_material_app.dart';

class _RunningEngineUi extends EngineUi {
  @override
  EngineUiSnapshot build() =>
      const EngineUiSnapshot(running: true, trackPaths: {});
}

FakeLibraryTransport _trackEditTransport(LibraryTrackSummary track) {
  final transport = FakeLibraryTransport(getTrackHandler: (id) async => track);
  addTearDown(transport.dispose);
  return transport;
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
    int deckCount = 2,
    SelectStyleSetting selectStyle = SelectStyleSetting.auto,
    FakeLibraryTransport? transport,
    Future<FakeLibraryTransport>? transportFuture,
  }) async {
    final theme = MixarThemeData.dark();
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          driveResolvedByPathProvider.overrideWith(
            (ref) async => const <String, LibraryTrackSummary>{},
          ),
          appSettingsProvider.overrideWith(
            (ref) async => copyAppSettings(
              defaultAppSettings(),
              selectStyle: selectStyle,
              numDecks: deckCount,
            ),
          ),
          if (transportFuture != null)
            libraryTransportProvider.overrideWith((ref) => transportFuture)
          else if (transport != null)
            libraryTransportProvider.overrideWith((ref) async => transport),
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

  testWidgets('desktop menu rows are divided by hairlines', (tester) async {
    final theme = MixarThemeData.dark();
    await pumpMenu(tester);
    await tester.tap(find.byIcon(LucideIcons.ellipsisVertical));
    await tester.pumpAndSettle();

    final horizontal = find.byWidgetPredicate(
      (w) => w is MDivider && w.axis == Axis.horizontal,
    );
    final dividers = tester.widgetList<MDivider>(horizontal).toList();
    expect(dividers.length, greaterThanOrEqualTo(3));
    for (final divider in dividers) {
      expect(divider.color, theme.colors.border);
      expect(divider.padding, EdgeInsets.zero);
    }

    // At least one rule falls between two action rows, not only at a group
    // boundary or inside the A/B track.
    final analyzeBottom = tester.getBottomLeft(find.text('Analyze')).dy;
    final generateTop = tester.getTopLeft(find.text('Generate stems')).dy;
    final betweenRows = Iterable<int>.generate(horizontal.evaluate().length)
        .any((i) {
          final y = tester.getCenter(horizontal.at(i)).dy;
          return y > analyzeBottom && y < generateTop;
        });
    expect(betweenRows, isTrue);
  });

  testWidgets('Analyze passes the replacement opt-in to the library', (
    tester,
  ) async {
    final transport = FakeLibraryTransport();
    addTearDown(transport.dispose);
    await pumpMenu(tester, transport: transport);
    await tester.tap(find.byIcon(LucideIcons.ellipsisVertical));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Analyze'));
    await tester.pumpAndSettle();

    final checkbox = find.byType(MixarCheckbox);
    expect(tester.widget<MixarCheckbox>(checkbox).value, isFalse);

    await tester.tap(find.byType(MixarCheckbox));
    await tester.pump();
    await tester.tap(find.text('Start analysis'));
    await tester.pumpAndSettle();
    expect(transport.analyzeTrackCalls, [(trackId: 't1', force: true)]);
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

  testWidgets('mobile menu rows are flat, full-bleed and aligned', (
    tester,
  ) async {
    await pumpMenu(tester, selectStyle: SelectStyleSetting.mobile);
    await tester.pumpAndSettle();
    await tester.tap(find.byIcon(LucideIcons.ellipsisVertical));
    await tester.pumpAndSettle();

    // Section label, action rows and the dialog title share one inset edge.
    final titleLeft = tester.getTopLeft(find.text('Track actions')).dx;
    final labelLeft = tester.getTopLeft(find.text('Load to deck')).dx;
    final analyzeLeft = tester.getTopLeft(find.text('Analyze')).dx;
    expect(labelLeft, moreOrLessEquals(titleLeft, epsilon: 0.5));
    expect(analyzeLeft, moreOrLessEquals(titleLeft, epsilon: 0.5));

    // The A/B control's track keeps the same inset.
    final track = find
        .descendant(
          of: find.byType(MixarMenuSegments),
          matching: find.byType(DecoratedBox),
        )
        .first;
    expect(
      tester.getRect(track).left,
      moreOrLessEquals(analyzeLeft, epsilon: 0.5),
    );
    // Deck buttons keep the bordered track they have in a desktop dropdown.
    expect(
      (tester.widget<DecoratedBox>(track).decoration as BoxDecoration).border,
      isNotNull,
    );

    // No per-row frames: rows are separated by hairlines only.
    final analyzeItem = find.ancestor(
      of: find.text('Analyze'),
      matching: find.byType(MixarMenuItem),
    );
    expect(analyzeItem, findsOneWidget);
    expect(
      find.descendant(
        of: analyzeItem,
        matching: find.byWidgetPredicate(
          (w) =>
              w is DecoratedBox &&
              w.decoration is BoxDecoration &&
              (w.decoration as BoxDecoration).border != null,
        ),
      ),
      findsNothing,
    );
    expect(
      find.byWidgetPredicate((w) => w is MDivider && w.axis == Axis.horizontal),
      findsWidgets,
    );
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

  testWidgets('Edit… opens the track edit drawer', (tester) async {
    final transport = _trackEditTransport(track);

    await pumpMenu(tester, transport: transport);
    await tester.tap(find.byIcon(LucideIcons.ellipsisVertical));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Edit…'));
    await tester.pumpAndSettle();

    // The drawer form is up with the track's fields...
    expect(find.text('Title'), findsOneWidget);
    expect(find.text('ISRC'), findsOneWidget);
    // ...and it is the row's track that was fetched, not some other id.
    expect(transport.getTrackCalls, ['t1']);
  });

  testWidgets('Edit… survives the menu closing before the transport resolves', (
    tester,
  ) async {
    final transport = _trackEditTransport(track);
    final gate = Completer<FakeLibraryTransport>();

    await pumpMenu(tester, transportFuture: gate.future);
    await tester.tap(find.byIcon(LucideIcons.ellipsisVertical));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Edit…'));
    await tester.pumpAndSettle();

    // The menu (and its ref/context) is gone, and the drawer is not up yet.
    expect(find.text('Title'), findsNothing);

    gate.complete(transport);
    await tester.pumpAndSettle();

    expect(find.text('Title'), findsOneWidget, reason: 'drawer opens');
    expect(transport.getTrackCalls, ['t1']);
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

  testWidgets('four-deck profile exposes load actions for C and D', (
    tester,
  ) async {
    await pumpMenu(tester, running: true, deckCount: 4);
    await tester.tap(find.byIcon(LucideIcons.ellipsisVertical));
    await tester.pumpAndSettle();

    expect(_loadSegment(tester, 'C').onPress, isNotNull);
    expect(_loadSegment(tester, 'D').onPress, isNotNull);
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
