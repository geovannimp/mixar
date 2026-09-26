import 'package:flutter/gestures.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/library/providers.dart';
import 'package:gui_flutter/library/track_table_pane.dart';
import 'package:gui_flutter/settings/settings_defaults.dart';
import 'package:gui_flutter/settings/settings_providers.dart';
import 'package:gui_flutter/shell/m_loader.dart';
import 'package:gui_flutter/shell/desktop.dart';
import 'package:gui_flutter/shell/mixar_menu.dart';
import 'package:gui_flutter/shell/material_theme.dart';
import 'package:gui_flutter/shell/mixar_theme.dart';
import 'package:gui_flutter/src/rust/api/library.dart';
import 'package:material_ui/material_ui.dart';
import 'package:trina_grid/trina_grid.dart';

import 'support/mixar_material_app.dart';

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

  Future<ProviderContainer> pumpTable(WidgetTester tester) async {
    debugOverrideDesktopWindow = false;
    addTearDown(() => debugOverrideDesktopWindow = null);
    final theme = MixarThemeData.light();
    tester.view.physicalSize = const Size(900, 600);
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
          home: const Scaffold(
            body: SizedBox(width: 900, height: 600, child: TrackTablePane()),
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
      expect(find.byType(FractionallySizedBox), findsNothing);

      container.read(trackProgressProvider.notifier).set(track.id, 'bpm', 0.42);
      await tester.pump();
      expect(find.text('Detecting BPM 42%'), findsOneWidget);
      expect(find.text('Demo Track'), findsOneWidget);
      expect(find.byType(FractionallySizedBox), findsOneWidget);

      // Stems open a second lane: both jobs report at once.
      container
          .read(trackProgressProvider.notifier)
          .set(track.id, 'stems_separate', 0.5);
      await tester.pump();
      expect(find.text('Detecting BPM 42%'), findsOneWidget);
      expect(find.text('Separating stems 50%'), findsOneWidget);
      // Only the working row is decorated.
      expect(find.text('Other Track'), findsOneWidget);
      // One bar per job.
      expect(find.byType(FractionallySizedBox), findsNWidgets(2));

      // Finishing stems drops only that lane.
      container
          .read(trackProgressProvider.notifier)
          .set(track.id, 'stems_ready', null);
      await tester.pump();
      expect(find.text('Separating stems 50%'), findsNothing);
      expect(find.text('Detecting BPM 42%'), findsOneWidget);
      expect(find.byType(FractionallySizedBox), findsOneWidget);
      expect(find.text('Demo Track'), findsOneWidget);
    },
  );

  testWidgets('the 3-dot menu stays available while a track is busy', (
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
    expect(find.byType(MLoader), findsNothing);
  });

  testWidgets('one loader per busy job, all inside the pill', (tester) async {
    final container = await pumpTable(tester);

    // Queued analysis: the pill owns the loader, the actions cell keeps its
    // normal icon.
    container.read(analyzingTrackIdsProvider.notifier).add(track.id);
    await tester.pump();
    expect(find.text('Analyzing'), findsOneWidget);
    expect(find.byType(MLoader), findsOneWidget);

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
    expect(find.byType(MLoader), findsNWidgets(2));

    // A determinate phase drops that lane's loader.
    container
        .read(trackProgressProvider.notifier)
        .set(track.id, 'stems_separate', 0.4);
    await tester.pump();
    expect(find.text('Separating stems 40%'), findsOneWidget);
    expect(find.byType(MLoader), findsOneWidget);
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
    final bar = tester.getRect(find.byType(FractionallySizedBox));
    expect(
      rowSlotBottom - bar.bottom,
      closeTo(manager.configuration.style.cellHorizontalBorderWidth, 0.01),
    );
    expect(bar.height, 2);
  });
}
