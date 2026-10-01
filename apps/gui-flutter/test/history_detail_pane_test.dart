import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/library/history_detail_pane.dart';
import 'package:gui_flutter/library/history_providers.dart';
import 'package:gui_flutter/library/library_list_chrome.dart';
import 'package:gui_flutter/library/providers.dart';
import 'package:gui_flutter/library/track_list.dart';
import 'package:gui_flutter/mixer/engine_providers.dart';
import 'package:gui_flutter/mixer/key_format.dart';
import 'package:gui_flutter/mixer/track_drag.dart';
import 'package:gui_flutter/settings/settings_defaults.dart';
import 'package:gui_flutter/settings/settings_providers.dart';
import 'package:gui_flutter/shell/material_theme.dart';
import 'package:gui_flutter/shell/mixar_theme.dart';
import 'package:gui_flutter/src/rust/api/library.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';
import 'package:material_ui/material_ui.dart';
import 'package:super_drag_and_drop/super_drag_and_drop.dart';

import 'support/mixar_material_app.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  const session = HistorySessionSummary(
    id: 's1',
    title: 'Friday set',
    startedAt: '2026-10-01T20:00:00',
    lastActivityAt: '2026-10-01T21:00:00',
    closed: false,
    entryCount: 2,
  );
  const entry = HistoryEntryInfo(
    id: 'e1',
    deck: 0,
    trackId: 't1',
    location: 'file:///tmp/samples/demo.wav',
    title: 'Demo Track',
    artist: 'Artist',
    album: 'Album',
    bpm: 128,
    key: '8A',
    isrc: 'USABC1234567',
    startedAt: '2026-10-01T20:14:00',
    endedAt: '2026-10-01T20:18:00',
    playedDurationMs: 222000,
  );
  const entryB = HistoryEntryInfo(
    id: 'e2',
    deck: 1,
    location: '/tmp/samples/other.wav',
    title: 'Other Track',
    artist: 'Someone Else',
    startedAt: '2026-10-01T20:20:00',
    playedDurationMs: 60000,
  );

  var pumpCount = 0;

  Future<ProviderContainer> pumpPane(
    WidgetTester tester, {
    List<HistorySessionSummary> sessions = const [session],
    List<HistoryEntryInfo> entries = const [entry, entryB],
    double width = 1000,
    double height = 600,
  }) async {
    final theme = MixarThemeData.light();
    tester.view.physicalSize = Size(width, height);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);

    await tester.pumpWidget(
      ProviderScope(
        // A fresh key per pump: `pumpWidget` otherwise reuses the container, so
        // a second call in the same test would keep the first call's overrides.
        key: ValueKey(pumpCount++),
        overrides: [
          historySessionsProvider.overrideWith((ref) async => sessions),
          historyEntriesProvider.overrideWith((ref) async => entries),
          appSettingsProvider.overrideWith((ref) async => defaultAppSettings()),
        ],
        child: MaterialApp(
          theme: materialUiThemeFromMixar(theme),
          builder: mixarMaterialAppBuilder(theme),
          home: Scaffold(
            body: SizedBox(
              width: width,
              height: height,
              child: const HistoryDetailPane(),
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();
    return ProviderScope.containerOf(
      tester.element(find.byType(HistoryDetailPane)),
    );
  }

  testWidgets('renders one list row per entry', (tester) async {
    await pumpPane(tester);

    expect(find.byType(TrackListRow), findsNWidgets(2));
    expect(find.byType(ListView), findsOneWidget);
    expect(find.text('Demo Track'), findsOneWidget);
    expect(find.text('Other Track'), findsOneWidget);
  });

  testWidgets('comfortable rows surface every snapshot field', (tester) async {
    await pumpPane(tester, width: 1100, entries: const [entry]);

    final row = find.byType(TrackListRow);
    // Title + pills.
    for (final label in [
      'Demo Track',
      'Artist',
      'Album',
      'demo.wav',
      'USABC1234567',
      '20:14 → 20:18',
      '3:42',
    ]) {
      expect(find.text(label), findsOneWidget, reason: 'missing "$label"');
    }
    // BPM and key live in the shared trailing group.
    final trailing = find.descendant(
      of: row,
      matching: find.byKey(kTrailingMetaKey),
    );
    expect(
      find.descendant(
        of: trailing,
        matching: find.text(formatDeckKey(entry.key, KeyDisplayMode.musical)),
      ),
      findsOneWidget,
    );
    expect(
      find.descendant(of: trailing, matching: find.text('128.0 BPM')),
      findsOneWidget,
    );
  });

  testWidgets('rows show the deck letter and position', (tester) async {
    await pumpPane(tester, width: 1100);

    // B (deck 1) is 'Other Track' at position #2.
    expect(find.text('B'), findsOneWidget);
    expect(find.text('A'), findsOneWidget);
    expect(find.text('1'), findsOneWidget);
    expect(find.text('2'), findsOneWidget);
    expect(find.bySemanticsLabel('Deck B'), findsOneWidget);
  });

  testWidgets('metadata text is hung left to align with the title', (
    tester,
  ) async {
    await pumpPane(tester, width: 1100, entries: const [entry]);

    // The first metadata pill carries the same 8px inset the others do. With
    // no visible chip background that inset reads as a bare indent, so the
    // whole metadata group is shifted back to line up with the title.
    final titleLeft = tester.getTopLeft(find.text('Demo Track')).dx;
    final firstPillLeft = tester.getTopLeft(find.text('Artist')).dx;
    expect(firstPillLeft, closeTo(titleLeft, 0.5));

    // The reported case: an entry with no artist metadata puts the file name
    // first, and it lines up too.
    await pumpPane(
      tester,
      width: 1100,
      entries: const [
        HistoryEntryInfo(
          id: 'e1',
          deck: 0,
          location: 'file:///tmp/samples/palawan.opus',
          title: 'Palawan',
          startedAt: '2026-10-01T10:26:00',
          endedAt: '2026-10-01T10:27:00',
          playedDurationMs: 39000,
        ),
      ],
    );
    final bareTitleLeft = tester.getTopLeft(find.text('Palawan')).dx;
    final fileLeft = tester.getTopLeft(find.text('palawan.opus')).dx;
    expect(fileLeft, closeTo(bareTitleLeft, 0.5));
  });

  testWidgets('clicking a row focuses it and highlights only that row', (
    tester,
  ) async {
    final container = await pumpPane(tester);
    final theme = MixarThemeData.light();
    Color rowColor(int index) {
      final box = tester.widget<DecoratedBox>(
        find
            .descendant(
              of: find.byType(TrackListRow).at(index),
              matching: find.byType(DecoratedBox),
            )
            .first,
      );
      return (box.decoration as BoxDecoration).color!;
    }

    // The first row is focused by default, exactly as the track list behaves.
    expect(container.read(focusedTrackRowIndexProvider), 0);
    expect(rowColor(0), libraryListSelectedRowColor(theme));
    expect(rowColor(1), theme.colors.card);

    await tester.tap(find.text('Other Track'));
    await tester.pumpAndSettle();

    expect(container.read(focusedTrackRowIndexProvider), 1);
    expect(rowColor(1), libraryListSelectedRowColor(theme));
    expect(rowColor(0), theme.colors.card);
  });

  testWidgets('row drag attaches after the engine starts', (tester) async {
    final container = await pumpPane(tester);
    expect(find.byType(DragItemWidget), findsNothing);

    container.read(engineUiProvider.notifier).setRunning(true);
    await tester.pumpAndSettle();

    expect(find.byType(DragItemWidget), findsWidgets);
  });

  testWidgets('arrow keys move the focused row, like the track list', (
    tester,
  ) async {
    final container = await pumpPane(tester);
    await tester.tap(find.text('Demo Track'));
    await tester.pumpAndSettle();
    expect(container.read(focusedTrackRowIndexProvider), 0);

    await tester.sendKeyEvent(LogicalKeyboardKey.end);
    await tester.pump();
    expect(container.read(focusedTrackRowIndexProvider), 1);

    await tester.sendKeyEvent(LogicalKeyboardKey.home);
    await tester.pump();
    expect(container.read(focusedTrackRowIndexProvider), 0);

    await tester.sendKeyEvent(LogicalKeyboardKey.arrowDown);
    await tester.pump();
    expect(container.read(focusedTrackRowIndexProvider), 1);
  });

  testWidgets('a row opens the shared actions menu', (tester) async {
    await pumpPane(tester);
    // Scope to the row: the toolbar's session menu uses the same ⋯ icon.
    await tester.tap(
      find.descendant(
        of: find.byType(TrackListRow).first,
        matching: find.byIcon(LucideIcons.ellipsisVertical),
      ),
    );
    await tester.pumpAndSettle();

    expect(find.text('Load to deck'), findsOneWidget);
  });

  testWidgets('publishes the focused row payload for controller load', (
    tester,
  ) async {
    final container = await pumpPane(tester);
    expect(
      container.read(focusedTrackPayloadProvider).payload,
      payloadFromHistoryEntry(entry),
    );

    // The same global payload the library list publishes, so MIDI "load focused
    // row" acts on the history row that is highlighted.
    container.read(focusedTrackRowIndexProvider.notifier).navigate(1);
    await tester.pump();
    expect(
      container.read(focusedTrackPayloadProvider).payload,
      payloadFromHistoryEntry(entryB),
    );
  });

  testWidgets('clears the focused payload when the list unmounts', (
    tester,
  ) async {
    final theme = MixarThemeData.light();
    final scopeKey = GlobalKey();
    Widget app({required bool showList}) => ProviderScope(
      key: scopeKey,
      overrides: [
        historySessionsProvider.overrideWith((ref) async => const [session]),
        historyEntriesProvider.overrideWith(
          (ref) async => const [entry, entryB],
        ),
        appSettingsProvider.overrideWith((ref) async => defaultAppSettings()),
      ],
      child: MaterialApp(
        theme: materialUiThemeFromMixar(theme),
        builder: mixarMaterialAppBuilder(theme),
        home: Scaffold(
          body: showList
              ? const SizedBox(
                  width: 800,
                  height: 400,
                  child: HistoryDetailPane(),
                )
              : const SizedBox.shrink(),
        ),
      ),
    );

    await tester.pumpWidget(app(showList: true));
    await tester.pumpAndSettle();
    final container = ProviderScope.containerOf(
      tester.element(find.byType(HistoryDetailPane)),
    );
    expect(container.read(focusedTrackPayloadProvider).payload, isNotNull);

    await tester.pumpWidget(app(showList: false));
    await tester.pumpAndSettle();
    // A controller "load" after teardown must no-op, not load a stale row.
    expect(container.read(focusedTrackPayloadProvider).payload, isNull);
    expect(container.read(focusedTrackPayloadProvider).owner, isNull);
  });

  testWidgets('the density button switches layout and resets on a second '
      'press', (tester) async {
    final container = await pumpPane(tester);
    expect(
      container.read(libraryRowDensityProvider),
      LibraryRowDensity.comfortable,
    );
    expect(
      tester.getSize(find.byType(TrackListRow).first).height,
      LibraryRowDensity.comfortable.height,
    );

    await tester.tap(find.bySemanticsLabel('Toggle row layout'));
    await tester.pumpAndSettle();
    expect(
      container.read(libraryRowDensityProvider),
      LibraryRowDensity.compact,
    );
    expect(
      tester.getSize(find.byType(TrackListRow).first).height,
      LibraryRowDensity.compact.height,
    );

    await tester.tap(find.bySemanticsLabel('Toggle row layout'));
    await tester.pumpAndSettle();
    expect(
      container.read(libraryRowDensityProvider),
      LibraryRowDensity.comfortable,
    );
  });

  testWidgets('compact hides the pills and keeps the length in its trailing '
      'group', (tester) async {
    final container = await pumpPane(tester);
    expect(find.byType(Wrap), findsNWidgets(2));

    container
        .read(libraryRowDensityOverrideProvider.notifier)
        .set(LibraryRowDensity.compact);
    await tester.pumpAndSettle();

    expect(find.byType(Wrap), findsNothing);
    expect(find.text('Artist'), findsNothing);
    // Length moved into the trailing group, alongside BPM and key.
    final trailing = find.byKey(kTrailingMetaKey);
    expect(
      find.descendant(of: trailing, matching: find.text('3:42')),
      findsOneWidget,
    );
  });

  testWidgets('the filter narrows the list', (tester) async {
    final container = await pumpPane(tester);
    expect(find.byType(TrackListRow), findsNWidgets(2));

    container.read(historyEntryFilterProvider.notifier).set('Other');
    await tester.pumpAndSettle();

    expect(find.byType(TrackListRow), findsOneWidget);
    expect(find.text('Other Track'), findsOneWidget);
    expect(find.text('Demo Track'), findsNothing);
  });

  testWidgets('an unmatched filter shows the empty-filter message', (
    tester,
  ) async {
    final container = await pumpPane(tester);
    container.read(historyEntryFilterProvider.notifier).set('zzz');
    await tester.pumpAndSettle();

    expect(find.text('No matching entries'), findsOneWidget);
    expect(find.byType(TrackListRow), findsNothing);
  });

  testWidgets('an empty session shows the no-plays message', (tester) async {
    await pumpPane(tester, entries: const []);
    expect(find.text('No plays logged in this session'), findsOneWidget);
  });

  testWidgets('no session shows the select-session message', (tester) async {
    await pumpPane(tester, sessions: const []);
    expect(find.text('Select a history session'), findsOneWidget);
  });

  testWidgets('pills wrap to two runs without clipping the last one', (
    tester,
  ) async {
    final overflows = <String>[];
    final previous = FlutterError.onError;
    FlutterError.onError = (details) {
      final text = details.toString();
      if (text.contains('overflowed')) overflows.add(text);
      previous?.call(details);
    };
    try {
      for (final width in [1000.0, 1400.0]) {
        await pumpPane(
          tester,
          width: width,
          entries: const [
            HistoryEntryInfo(
              id: 'e1',
              deck: 0,
              location: '/tmp/samples/a-really-long-file-name-goes-here.wav',
              title: 'A Long Title For Wrapping',
              artist: 'A Really Very Extremely Long Artist Name Indeed',
              album: 'A Long Album Title',
              isrc: 'USABC1234567',
              startedAt: '2026-10-01T20:14:00',
              endedAt: '2026-10-01T20:18:00',
              playedDurationMs: 222000,
            ),
          ],
        );

        final row = tester.getRect(find.byType(TrackListRow));
        for (final label in [
          'A Really Very Extremely Long Artist Name Indeed',
          'A Long Album Title',
          'USABC1234567',
        ]) {
          final pill = find.text(label);
          expect(pill, findsOneWidget, reason: '$label missing at $width');
          expect(
            tester.getRect(pill).bottom,
            lessThanOrEqualTo(row.bottom + 0.5),
            reason: '$label clipped out of the row at $width',
          );
        }
      }
    } finally {
      FlutterError.onError = previous;
    }
    expect(overflows, isEmpty, reason: 'overflowed: $overflows');
  });

  group('payloadFromHistoryEntry', () {
    test('uses the library id and normalizes the file URI', () {
      final payload = payloadFromHistoryEntry(entry);

      expect(payload.source, TrackDragSource.library);
      expect(payload.trackId, 't1');
      expect(payload.path, '/tmp/samples/demo.wav');
      expect(payload.title, 'Demo Track');
    });

    test('falls back to the path when the entry has no track id', () {
      final payload = payloadFromHistoryEntry(entryB);

      expect(payload.source, TrackDragSource.filesystem);
      expect(payload.trackId, isNull);
      expect(payload.path, '/tmp/samples/other.wav');
      expect(payload.title, 'Other Track');
    });
  });

  group('formatHistoryPlaySpan', () {
    test('drops the duplicated date for a single-day play', () {
      expect(
        formatHistoryPlaySpan('2026-10-01T20:14:00', '2026-10-01T20:18:00'),
        '20:14 → 20:18',
      );
    });

    test('keeps both full timestamps when the play crosses midnight', () {
      expect(
        formatHistoryPlaySpan('2026-10-01T23:55:00', '2026-10-02T00:05:00'),
        '2026-10-01 23:55 → 2026-10-02 00:05',
      );
    });

    test('marks an open entry as live', () {
      expect(
        formatHistoryPlaySpan('2026-10-01T20:14:00', null),
        '2026-10-01 20:14 → live',
      );
    });

    test('falls back gracefully on unparseable timestamps', () {
      expect(
        formatHistoryPlaySpan('not-a-date', 'also-not-a-date'),
        'not-a-date → also-not-a-date',
      );
    });
  });
}
