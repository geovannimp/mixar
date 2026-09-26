import 'package:flutter/gestures.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/library/providers.dart';
import 'package:gui_flutter/library/track_table_pane.dart';
import 'package:gui_flutter/settings/settings_defaults.dart';
import 'package:gui_flutter/settings/settings_providers.dart';
import 'package:gui_flutter/shell/app_button.dart';
import 'package:gui_flutter/shell/desktop.dart';
import 'package:gui_flutter/shell/m_loader.dart';
import 'package:gui_flutter/shell/material_theme.dart';
import 'package:gui_flutter/shell/mixar_menu.dart';
import 'package:gui_flutter/shell/mixar_theme.dart';
import 'package:gui_flutter/src/rust/api/library.dart';
import 'package:material_ui/material_ui.dart';
import 'package:trina_grid/trina_grid.dart';

import 'support/mixar_material_app.dart';

// Both widget types are used elsewhere in the app (m_tabs.dart renders
// FractionallySizedBox), so a global count would break for reasons unrelated
// to the overlay. Scope to the pane under test.
Finder barsInPane() => find.descendant(
  of: find.byType(TrackTablePane),
  matching: find.byType(FractionallySizedBox),
);

Finder loadersInPane() => find.descendant(
  of: find.byType(TrackTablePane),
  matching: find.byType(MLoader),
);

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  const collection = LibraryCollectionSummary(
    id: 'c1',
    name: 'samples',
    kind: 'folder',
    path: '/tmp/samples',
    trackCount: 2,
  );
  const track = LibraryTrackSummary(
    id: 't1',
    displayName: 'Demo Track',
    title: 'Demo Track',
    bpm: 128,
    durationMs: 180000,
    path: '/tmp/samples/demo.wav',
  );
  const trackB = LibraryTrackSummary(
    id: 't2',
    displayName: 'Other Track',
    title: 'Other Track',
    path: '/tmp/samples/other.wav',
  );

  Future<ProviderContainer> pumpTable(
    WidgetTester tester, {
    double width = 900,
  }) async {
    debugOverrideDesktopWindow = false;
    addTearDown(() => debugOverrideDesktopWindow = null);
    final theme = MixarThemeData.light();
    tester.view.physicalSize = Size(width, 600);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);

    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          collectionsProvider.overrideWith((ref) async => [collection]),
          collectionTracksProvider.overrideWith((ref) async => [track, trackB]),
          libraryEventsBootstrapProvider.overrideWith((ref) {}),
          appSettingsProvider.overrideWith((ref) async => defaultAppSettings()),
        ],
        child: MaterialApp(
          theme: materialUiThemeFromMixar(theme),
          builder: mixarMaterialAppBuilder(theme),
          home: Scaffold(
            body: SizedBox(
              width: width,
              height: 600,
              child: const TrackTablePane(),
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();
    return ProviderScope.containerOf(
      tester.element(find.byType(TrackTablePane)),
    );
  }

  testWidgets(
    'analysis and stem phases render as a row overlay, not in the title',
    (tester) async {
      final container = await pumpTable(tester);
      expect(find.text('Analyzing'), findsNothing);

      // No phase event yet, but the track is queued: indeterminate, no bar.
      container.read(analyzingTrackIdsProvider.notifier).add(track.id);
      await tester.pump();
      expect(find.text('Analyzing'), findsOneWidget);
      expect(find.text('Demo Track'), findsOneWidget);
      // A percentage is only drawn when the engine actually reported one.
      expect(barsInPane(), findsNothing);

      container.read(trackProgressProvider.notifier).set(track.id, 'bpm', 0.42);
      await tester.pump();
      expect(find.text('Detecting BPM 42%'), findsOneWidget);
      expect(find.text('Demo Track'), findsOneWidget);
      expect(barsInPane(), findsOneWidget);

      // Stems open a second lane: both jobs report at once.
      container
          .read(trackProgressProvider.notifier)
          .set(track.id, 'stems_separate', 0.5);
      await tester.pump();
      expect(find.text('Detecting BPM 42%'), findsOneWidget);
      expect(find.text('Separating stems 50%'), findsOneWidget);
      // The idle row still renders, and contributes no pill or bar: exactly
      // two bars exist, one per job on the working row.
      expect(find.text('Other Track'), findsOneWidget);
      expect(barsInPane(), findsNWidgets(2));

      // Finishing stems drops only that lane.
      container
          .read(trackProgressProvider.notifier)
          .set(track.id, 'stems_ready', null);
      await tester.pump();
      expect(find.text('Separating stems 50%'), findsNothing);
      expect(find.text('Detecting BPM 42%'), findsOneWidget);
      expect(barsInPane(), findsOneWidget);
      expect(find.text('Demo Track'), findsOneWidget);
    },
  );

  testWidgets('row and cell menus stay available while a track is busy', (
    tester,
  ) async {
    final container = await pumpTable(tester);
    container.read(analyzingTrackIdsProvider.notifier).add(track.id);
    container.read(trackProgressProvider.notifier).set(track.id, 'bpm', 0.5);
    await tester.pump();

    // Right-click still opens, and the running job disables only its own item.
    await tester.tap(find.text('Demo Track'), buttons: kSecondaryButton);
    await tester.pumpAndSettle();
    expect(find.text('Load to deck'), findsOneWidget);
    expect(find.text('Analyzing…'), findsOneWidget);
    expect(
      tester
          .widget<MixarMenuItem>(
            find.widgetWithText(MixarMenuItem, 'Analyzing…'),
          )
          .enabled,
      isFalse,
    );
    // Stems have not run, so that action is still offered.
    expect(
      tester
          .widget<MixarMenuItem>(
            find.widgetWithText(MixarMenuItem, 'Generate stems'),
          )
          .enabled,
      isTrue,
    );
    // Determinate analysis: no spinner anywhere, least of all in the cell.
    expect(loadersInPane(), findsNothing);

    // The actions cell 3-dot itself also opens while analyzing — previously it
    // swapped to a loader and stopped responding.
    await tester.tap(
      find
          .byWidgetPredicate(
            (w) => w is AppButton && w.semanticsLabel == 'Track actions',
          )
          // First row is `track`, the one under analysis.
          .first,
    );
    await tester.pumpAndSettle();
    expect(find.text('Analyzing…'), findsOneWidget);
  });

  testWidgets('one loader per busy job, all inside the pill', (tester) async {
    final container = await pumpTable(tester);

    // Queued analysis: the pill owns the loader, the actions cell keeps its
    // normal icon.
    container.read(analyzingTrackIdsProvider.notifier).add(track.id);
    await tester.pump();
    expect(find.text('Analyzing'), findsOneWidget);
    expect(loadersInPane(), findsOneWidget);

    // Two lanes open: one loader each, and the 3-dot cell is not a third.
    container
        .read(trackProgressProvider.notifier)
        .set(track.id, 'stems_queued', null);
    container
        .read(stemGeneratingTrackIdsProvider.notifier)
        .setGenerating(track.id, true);
    await tester.pump();
    expect(find.text('Analyzing'), findsOneWidget);
    expect(find.text('Queuing stems'), findsOneWidget);
    expect(loadersInPane(), findsNWidgets(2));

    // A determinate phase drops that lane's loader.
    container
        .read(trackProgressProvider.notifier)
        .set(track.id, 'stems_separate', 0.4);
    await tester.pump();
    expect(find.text('Separating stems 40%'), findsOneWidget);
    expect(loadersInPane(), findsOneWidget);
  });

  testWidgets('progress ticks reuse grid rows instead of regenerating them', (
    tester,
  ) async {
    final container = await pumpTable(tester);
    final manager = tester
        .state<TrinaGridState>(find.byType(TrinaGrid))
        .stateManager;
    final before = List<TrinaRow<dynamic>>.from(manager.refRows);
    expect(before, hasLength(2));

    container
        .read(trackProgressProvider.notifier)
        .set(track.id, 'analyze', 0.1);
    container.read(analyzingTrackIdsProvider.notifier).add(track.id);
    await tester.pump();
    container
        .read(trackProgressProvider.notifier)
        .set(track.id, 'analyze', 0.9);
    await tester.pump();

    final after = manager.refRows;
    expect(after.first, same(before.first));
    expect(after.last, same(before.last));
    expect(find.text('Analyzing 90%'), findsOneWidget);
  });

  testWidgets('row actions menu tracks job state without a row rebuild', (
    tester,
  ) async {
    final container = await pumpTable(tester);
    TrackActionsMenu menu() =>
        tester.widget<TrackActionsMenu>(find.byType(TrackActionsMenu).first);

    expect(menu().analyzing, isFalse);
    expect(menu().stemsGenerating, isFalse);

    // Determinate so the overlay has no running spinner (pumpAndSettle-safe).
    container
        .read(trackProgressProvider.notifier)
        .set(track.id, 'stems_separate', 0.3);
    container
        .read(stemGeneratingTrackIdsProvider.notifier)
        .setGenerating(track.id, true);
    await tester.pump();
    expect(menu().stemsGenerating, isTrue);

    container
        .read(trackProgressProvider.notifier)
        .set(track.id, 'stems_ready', null);
    container
        .read(stemGeneratingTrackIdsProvider.notifier)
        .setGenerating(track.id, false);
    await tester.pump();
    expect(menu().stemsGenerating, isFalse);
  });

  testWidgets('pills stay right-aligned as lanes stack', (tester) async {
    final container = await pumpTable(tester);
    container.read(analyzingTrackIdsProvider.notifier).add(track.id);
    await tester.pump();
    final singleRight = tester.getRect(find.text('Analyzing')).right;

    container
        .read(trackProgressProvider.notifier)
        .set(track.id, 'stems_queued', null);
    container
        .read(stemGeneratingTrackIdsProvider.notifier)
        .setGenerating(track.id, true);
    await tester.pump();
    expect(find.text('Analyzing'), findsOneWidget);
    expect(find.text('Queuing stems'), findsOneWidget);

    // Adding a pill must not push the group off the right edge.
    expect(
      tester.getRect(find.text('Queuing stems')).right,
      closeTo(singleRight, 0.5),
    );
  });

  // The two bar-placement tests below assert against trina_grid's row-slot
  // math (bodyTopOffset + rowTotalHeight - cellHorizontalBorderWidth), so they
  // are coupled to that library's layout contract rather than to pixels we
  // choose. A trina_grid upgrade that changes those semantics is the expected
  // cause of failure here; do not loosen the tolerance to make them green.
  testWidgets('progress bars sit flush on the row content edge', (
    tester,
  ) async {
    final container = await pumpTable(tester);
    container.read(trackProgressProvider.notifier).set(track.id, 'bpm', 0.7);
    await tester.pump();

    final manager = tester
        .state<TrinaGridState>(find.byType(TrinaGrid))
        .stateManager;
    // Trina's row slot is rowHeight + a horizontal cell border; anchoring the
    // bar to the slot instead left a visible gap below it.
    final rowSlotBottom = manager.bodyTopOffset + manager.rowTotalHeight;
    final bar = tester.getRect(barsInPane());
    expect(
      rowSlotBottom - bar.bottom,
      closeTo(manager.configuration.style.cellHorizontalBorderWidth, 0.01),
    );
  });

  testWidgets('a bar ordinal ignores lanes that render no bar', (tester) async {
    final container = await pumpTable(tester);
    // Analysis reports no fraction, stems do. The single bar belongs on the
    // content edge, not one stride up.
    container
        .read(trackProgressProvider.notifier)
        .set(track.id, 'analyze', null);
    container
        .read(trackProgressProvider.notifier)
        .set(track.id, 'stems_separate', 0.5);
    await tester.pump();

    final manager = tester
        .state<TrinaGridState>(find.byType(TrinaGrid))
        .stateManager;
    final rowSlotBottom = manager.bodyTopOffset + manager.rowTotalHeight;
    final bars = barsInPane();
    expect(bars, findsOneWidget);
    expect(
      rowSlotBottom - tester.getRect(bars).bottom,
      closeTo(manager.configuration.style.cellHorizontalBorderWidth, 0.01),
    );
  });

  testWidgets('two long pills do not overflow a narrow pane', (tester) async {
    final overflows = <String>[];
    final container = await pumpTable(tester, width: 380);

    // Scoped to the pump that can overflow and restored in a finally, so the
    // process-global handler cannot leak into another test in this file.
    final previous = FlutterError.onError;
    FlutterError.onError = (d) {
      final text = d.toString();
      if (text.contains('overflowed')) overflows.add(text);
      // Always forward, including overflows: flutter_test asserts that a
      // handler which swallows an error leaves its bookkeeping inconsistent,
      // which replaces a readable failure with an internal assertion.
      previous?.call(d);
    };
    try {
      // Longest realistic label on both lanes at once.
      container.read(analyzingTrackIdsProvider.notifier).add(track.id);
      container
          .read(trackProgressProvider.notifier)
          .set(track.id, 'loudness', 1);
      container
          .read(trackProgressProvider.notifier)
          .set(track.id, 'stems_separate', 1);
      await tester.pump();
    } finally {
      FlutterError.onError = previous;
    }

    expect(find.text('Measuring loudness 100%'), findsOneWidget);
    expect(find.text('Separating stems 100%'), findsOneWidget);
    expect(overflows, isEmpty, reason: 'pills overflowed: $overflows');
  });

  testWidgets('a failed stem job clears only the stem lane', (tester) async {
    final container = await pumpTable(tester);
    container.read(analyzingTrackIdsProvider.notifier).add(track.id);
    container.read(trackProgressProvider.notifier).set(track.id, 'bpm', 0.6);
    container
        .read(trackProgressProvider.notifier)
        .set(track.id, 'stems_separate', 0.3);
    await tester.pump();
    expect(find.text('Detecting BPM 60%'), findsOneWidget);
    expect(find.text('Separating stems 30%'), findsOneWidget);
    expect(barsInPane(), findsNWidgets(2));

    // Failure drops the stem lane; the concurrent analysis job keeps running
    // and keeps its own pill and bar.
    container
        .read(trackProgressProvider.notifier)
        .set(track.id, 'stems_failed', null);
    await tester.pump();
    expect(find.text('Separating stems 30%'), findsNothing);
    expect(find.text('Detecting BPM 60%'), findsOneWidget);
    expect(barsInPane(), findsOneWidget);
    expect(find.text('Demo Track'), findsOneWidget);
  });

  testWidgets('a failed stem job with no analysis leaves the row clean', (
    tester,
  ) async {
    final container = await pumpTable(tester);
    container
        .read(stemGeneratingTrackIdsProvider.notifier)
        .setGenerating(track.id, true);
    container
        .read(trackProgressProvider.notifier)
        .set(track.id, 'stems_model', null);
    await tester.pump();
    expect(find.text('Loading stem model'), findsOneWidget);

    container
        .read(trackProgressProvider.notifier)
        .set(track.id, 'stems_failed', null);
    // The real LibraryEvt path clears stemGenerating alongside the lane
    // (_handleLibraryEvt). Mirror that: leaving it set is a state the overlay
    // never sees, and it would correctly re-synthesise a "Queuing stems" pill.
    container
        .read(stemGeneratingTrackIdsProvider.notifier)
        .setGenerating(track.id, false);
    await tester.pump();
    // No stranded pill, no queued-stem fallback, no bar.
    expect(find.text('Loading stem model'), findsNothing);
    expect(find.text('Queuing stems'), findsNothing);
    expect(barsInPane(), findsNothing);
  });

  test('a non-finite engine fraction is dropped, not shown as 100%', () {
    final container = ProviderContainer();
    addTearDown(container.dispose);
    final notifier = container.read(trackProgressProvider.notifier);

    // Asserted rather than force-unwrapped so a provider shape change fails
    // with the real reason instead of a null-check error.
    TrackProgressInfo onlyJob(String id) {
      final jobs = container.read(trackProgressProvider)[id];
      expect(jobs, isNotNull, reason: 'no job recorded for $id');
      expect(jobs, hasLength(1), reason: 'expected one lane for $id');
      return jobs!.single;
    }

    notifier.set('t1', 'stems_separate', double.nan);
    expect(onlyJob('t1').fraction, isNull);
    expect(onlyJob('t1').label, 'Separating stems');

    notifier.set('t1', 'stems_separate', double.infinity);
    expect(onlyJob('t1').fraction, isNull);

    notifier.set('t1', 'stems_separate', 0.25);
    expect(onlyJob('t1').label, 'Separating stems 25%');
  });

  test('an unchanged phase keeps the list identity so select can suppress', () {
    final container = ProviderContainer();
    addTearDown(container.dispose);
    final notifier = container.read(trackProgressProvider.notifier);

    notifier.set('t1', 'stems_separate', null);
    final first = container.read(trackProgressProvider)['t1'];
    // The engine emits this exact pair twice: once from the pre-call report
    // and once from the first progress tick.
    notifier.set('t1', 'stems_separate', null);
    expect(
      identical(container.read(trackProgressProvider)['t1'], first),
      isTrue,
    );

    // A real change still publishes.
    notifier.set('t1', 'stems_separate', 0.4);
    expect(
      identical(container.read(trackProgressProvider)['t1'], first),
      isFalse,
    );
    expect(container.read(trackProgressProvider)['t1']!.single.fraction, 0.4);
  });

  test('updating one lane keeps the lane order stable', () {
    final container = ProviderContainer();
    addTearDown(container.dispose);
    final notifier = container.read(trackProgressProvider.notifier);

    notifier.set('t1', 'bpm', 0.5);
    notifier.set('t1', 'stems_separate', 0.2);
    List<String?> phases() => [
      for (final job in container.read(trackProgressProvider)['t1']!) job.phase,
    ];
    expect(phases(), ['bpm', 'stems_separate']);

    // Re-reporting the first lane must not move it to the end of the list.
    notifier.set('t1', 'bpm', 0.9);
    expect(phases(), ['bpm', 'stems_separate']);
  });
}
