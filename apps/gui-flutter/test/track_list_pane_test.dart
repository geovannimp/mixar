import 'package:flutter/gestures.dart';

import 'package:flutter/rendering.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/theme/material_theme.dart';
import 'package:gui_flutter/core/utils/desktop.dart';
import 'package:gui_flutter/core/utils/key_format.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/providers/library/focused_load.dart';
import 'package:gui_flutter/providers/library/providers.dart';
import 'package:gui_flutter/providers/mixer/engine_providers.dart';
import 'package:gui_flutter/providers/settings/settings_providers.dart';
import 'package:gui_flutter/src/rust/api/library.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';
import 'package:gui_flutter/widgets/library/track_list_pane.dart';
import 'package:gui_flutter/core/utils/library_list_layout.dart';
import 'package:gui_flutter/widgets/library/track_list_row.dart';
import 'package:gui_flutter/widgets/library/track_title_line.dart';
import 'package:gui_flutter/widgets/shell/mixar_menu_item.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';
import 'package:material_ui/material_ui.dart';
import 'package:super_drag_and_drop/super_drag_and_drop.dart';

import 'helpers/collection_tracks_override.dart';

import 'support/border_helpers.dart';
import 'support/mixar_material_app.dart';

/// The row element whose text is [label], for the row-height assertions.
Finder rowByText(String label) =>
    find.ancestor(of: find.text(label), matching: find.byType(TrackListRow));

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  const collection = LibraryCollectionSummary(
    id: 'c1',
    name: 'samples',
    kind: 'folder',
    path: '/tmp/samples',
    trackCount: 1,
  );
  const track = LibraryTrackSummary(
    id: 't1',
    displayName: 'Demo Track',
    artist: 'Artist',
    title: 'Demo Track',
    bpm: 128,
    key: '8A',
    durationMs: 180000,
    path: '/tmp/samples/demo.wav',
  );
  const trackB = LibraryTrackSummary(
    id: 't2',
    displayName: 'Other Track',
    title: 'Other Track',
    bpm: 90,
    key: '3B',
    path: '/tmp/samples/other.wav',
  );
  const keyedTrack = LibraryTrackSummary(
    id: 't3',
    displayName: 'Keyed Track',
    title: 'Keyed Track',
    key: '8A',
    path: '/tmp/samples/keyed.wav',
  );

  var pumpCount = 0;

  Future<ProviderContainer> pumpList(
    WidgetTester tester, {
    List<LibraryTrackSummary> tracks = const [track],
    double width = 900,
    double height = 600,

    /// Stubs the engine transport to null so a load short-circuits instead of
    /// reaching the bridge. Lets a test observe which deck a load targeted.
    bool stubEngine = false,
  }) async {
    debugOverrideDesktopWindow = false;
    addTearDown(() => debugOverrideDesktopWindow = null);
    final theme = MixarThemeData.light();
    tester.view.physicalSize = Size(width, height);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);

    await tester.pumpWidget(
      ProviderScope(
        // A fresh key per pump: `pumpWidget` otherwise reuses the container,
        // so a second call in the same test would keep the first call's
        // overrides and the session-only density override.
        key: ValueKey(pumpCount++),
        overrides: [
          collectionsProvider.overrideWith((ref) async => [collection]),
          overrideCollectionTracksValue(tracks),
          libraryEventsBootstrapProvider.overrideWith((ref) {}),
          appSettingsProvider.overrideWith((ref) async => defaultAppSettings()),
          if (stubEngine)
            engineTransportProvider.overrideWith((ref) async => null),
        ],
        child: MaterialApp(
          theme: materialUiThemeFromMixar(theme),
          builder: mixarMaterialAppBuilder(theme),
          home: Scaffold(
            body: SizedBox(
              width: width,
              height: height,
              child: const TrackListPane(),
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();
    return ProviderScope.containerOf(
      tester.element(find.byType(TrackListPane)),
    );
  }

  testWidgets('row drag attaches after engine starts without changing '
      'collection', (tester) async {
    final container = await pumpList(tester);
    expect(find.text('Demo Track'), findsOneWidget);
    expect(find.byType(DragItemWidget), findsNothing);

    container.read(engineUiProvider.notifier).setRunning(true);
    await tester.pumpAndSettle();

    expect(find.byType(DragItemWidget), findsWidgets);
  });

  testWidgets('MIDI focus survives analysis-state row rebuild', (tester) async {
    final container = await pumpList(tester, tracks: const [track, trackB]);

    container.read(focusedTrackRowIndexProvider.notifier).navigate(1);
    await tester.pump();
    expect(container.read(focusedTrackRowIndexProvider), 1);

    container.read(analyzingTrackIdsProvider.notifier).add(track.id);
    await tester.pump();
    expect(container.read(focusedTrackRowIndexProvider), 1);
  });

  testWidgets('the cursor tint follows the pane that owns focus', (
    tester,
  ) async {
    final container = await pumpList(tester, tracks: const [track, trackB]);
    final theme = MixarThemeData.light();
    Color rowColor(int index) => fillOf(
      tester,
      find
          .descendant(
            of: find.byType(TrackListRow).at(index),
            matching: find.byType(DecoratedBox),
          )
          .first,
    );

    // The list owns focus on open, so its cursor wears the brand tint.
    expect(
      container.read(libraryFocusZoneProvider),
      LibraryFocusZone.trackList,
    );
    expect(rowColor(0), libraryListSelectedRowColor(theme));

    // A focus step to the sidebar dims the cursor here instead.
    container
        .read(libraryFocusZoneProvider.notifier)
        .set(LibraryFocusZone.sidebar);
    await tester.pump();
    expect(rowColor(0), libraryListCursorRowColor(theme));

    container
        .read(libraryFocusZoneProvider.notifier)
        .set(LibraryFocusZone.trackList);
    await tester.pump();
    expect(rowColor(0), libraryListSelectedRowColor(theme));
  });

  testWidgets('a focus step hands keyboard focus to the list', (tester) async {
    final container = await pumpList(tester, tracks: const [track, trackB]);
    // Whether the node holding keyboard focus sits inside the list pane.
    bool listHasKeyboardFocus() {
      final context = tester.binding.focusManager.primaryFocus?.context;
      return context != null &&
          context.findAncestorWidgetOfExactType<TrackListPane>() != null;
    }

    tester.binding.focusManager.primaryFocus?.unfocus();
    await tester.pump();
    expect(listHasKeyboardFocus(), isFalse);

    // Step away and back: the list starts focused, so stepping back is the only
    // transition that can hand it the keys.
    container
        .read(libraryFocusZoneProvider.notifier)
        .set(LibraryFocusZone.sidebar);
    await tester.pump();
    expect(listHasKeyboardFocus(), isFalse);

    container
        .read(libraryFocusZoneProvider.notifier)
        .set(LibraryFocusZone.trackList);
    await tester.pump();
    expect(listHasKeyboardFocus(), isTrue);

    // Focus moving to the sidebar gives the keys away.
    container
        .read(libraryFocusZoneProvider.notifier)
        .set(LibraryFocusZone.sidebar);
    await tester.pump();
    expect(listHasKeyboardFocus(), isFalse);
  });

  testWidgets('publishes the focused row payload for controller load', (
    tester,
  ) async {
    final container = await pumpList(tester, tracks: const [track, trackB]);

    // The row the list starts on, published for MIDI "load focused row".
    expect(
      container.read(focusedTrackPayloadProvider).payload,
      payloadFromListTrack(track, inLibrary: true),
    );

    container.read(focusedTrackRowIndexProvider.notifier).navigate(1);
    await tester.pump();
    expect(
      container.read(focusedTrackPayloadProvider).payload,
      payloadFromListTrack(trackB, inLibrary: true),
    );
  });

  testWidgets('right-click on a track row opens the actions menu', (
    tester,
  ) async {
    await pumpList(tester);

    await tester.tap(find.text('Demo Track'), buttons: kSecondaryButton);
    await tester.pumpAndSettle();
    expect(find.text('Load to deck'), findsOneWidget);
  });

  testWidgets('arrow keys move the focused row and clamp at both ends', (
    tester,
  ) async {
    final container = await pumpList(tester, tracks: const [track, trackB]);
    // Clicking a row grants the list keyboard focus, which is what the arrow
    // keys then drive.
    await tester.tap(find.text('Demo Track'));
    await tester.pumpAndSettle();
    expect(container.read(focusedTrackRowIndexProvider), 0);

    // End jumps to the last row.
    await tester.sendKeyEvent(LogicalKeyboardKey.end);
    await tester.pump();
    expect(container.read(focusedTrackRowIndexProvider), 1);
    // End again is a no-op rather than a wrap.
    await tester.sendKeyEvent(LogicalKeyboardKey.end);
    await tester.pump();
    expect(container.read(focusedTrackRowIndexProvider), 1);

    await tester.sendKeyEvent(LogicalKeyboardKey.home);
    await tester.pump();
    expect(container.read(focusedTrackRowIndexProvider), 0);
    await tester.sendKeyEvent(LogicalKeyboardKey.arrowUp);
    await tester.pump();
    expect(container.read(focusedTrackRowIndexProvider), 0);

    await tester.sendKeyEvent(LogicalKeyboardKey.arrowDown);
    await tester.pump();
    expect(container.read(focusedTrackRowIndexProvider), 1);
  });

  testWidgets('clicking a row focuses it', (tester) async {
    final container = await pumpList(tester, tracks: const [track, trackB]);
    await tester.tap(find.text('Other Track'));
    await tester.pump();
    expect(container.read(focusedTrackRowIndexProvider), 1);
  });

  testWidgets('Enter loads the focused row to deck A, Shift+Enter to deck B', (
    tester,
  ) async {
    // The only observable seam is the per-deck in-flight counter: the engine
    // transport is an opaque bridge object, so a null engine is stubbed in and
    // the load short-circuits after recording which deck it targeted. That
    // still proves the index was in range — an out-of-range row returns before
    // touching the counter at all.
    final container = await pumpList(
      tester,
      tracks: const [track, trackB],
      stubEngine: true,
    );

    final loadedDecks = <int>[];
    container.listen(deckLoadInFlightProvider, (_, next) {
      for (final deckId in next.keys) {
        if (!loadedDecks.contains(deckId)) {
          loadedDecks.add(deckId);
        }
      }
    });

    // Focus the second row, so a regression that loads index 0 is caught.
    await tester.tap(find.text('Other Track'));
    await tester.pumpAndSettle();
    expect(container.read(focusedTrackRowIndexProvider), 1);

    await tester.sendKeyEvent(LogicalKeyboardKey.enter);
    await tester.pumpAndSettle();
    expect(loadedDecks, [0], reason: 'Enter should load deck A');

    loadedDecks.clear();
    await tester.sendKeyDownEvent(LogicalKeyboardKey.shiftLeft);
    await tester.sendKeyEvent(LogicalKeyboardKey.enter);
    await tester.sendKeyUpEvent(LogicalKeyboardKey.shiftLeft);
    await tester.pumpAndSettle();
    expect(loadedDecks, [1], reason: 'Shift+Enter should load deck B');
  });

  testWidgets('the sort menu reorders the list', (tester) async {
    final container = await pumpList(tester, tracks: const [track, trackB]);
    expect(
      container.read(libraryTableTracksProvider).asData!.value.first.id,
      't1',
    );

    Finder sortOption(String label) => find.descendant(
      of: find.byType(MixarMenuItem),
      matching: find.text(label),
    );

    await tester.tap(find.bySemanticsLabel(RegExp('Sort tracks')));
    await tester.pumpAndSettle();
    expect(find.text('Ascending'), findsNothing);
    expect(find.text('Descending'), findsNothing);
    await tester.tap(sortOption('BPM'));
    await tester.pumpAndSettle();

    // 90 < 128, so the ascending default puts track B first.
    var sorted = container.read(libraryTableTracksProvider).asData!.value;
    expect(sorted.map((t) => t.id), ['t2', 't1']);
    expect(find.text('Other Track'), findsOneWidget);
    // Arrow points the way the list reads: A→Z / low→high is ↓, Z→A is ↑.
    final sortButton = find.bySemanticsLabel(RegExp('Sort tracks'));
    expect(
      find.descendant(
        of: sortButton,
        matching: find.byIcon(LucideIcons.arrowDown),
      ),
      findsOneWidget,
    );
    expect(
      find.descendant(
        of: sortButton,
        matching: find.byIcon(LucideIcons.arrowUp),
      ),
      findsNothing,
    );

    // Second click on the active field flips direction, like a column header.
    await tester.tap(sortButton);
    await tester.pumpAndSettle();
    await tester.tap(sortOption('BPM'));
    await tester.pumpAndSettle();
    sorted = container.read(libraryTableTracksProvider).asData!.value;
    expect(sorted.map((t) => t.id), ['t1', 't2']);
    expect(
      find.descendant(
        of: sortButton,
        matching: find.byIcon(LucideIcons.arrowUp),
      ),
      findsOneWidget,
    );
    expect(
      find.descendant(
        of: sortButton,
        matching: find.byIcon(LucideIcons.arrowDown),
      ),
      findsNothing,
    );
  });

  testWidgets(
    'the density button switches layout and resets on a second press',
    (tester) async {
      final container = await pumpList(tester);
      expect(
        container.read(libraryRowDensityProvider),
        LibraryRowDensity.comfortable,
      );
      expect(
        tester.getSize(rowByText('Demo Track')).height,
        LibraryRowDensity.comfortable.height,
      );

      await tester.tap(find.bySemanticsLabel('Toggle row layout'));
      await tester.pumpAndSettle();
      expect(
        container.read(libraryRowDensityProvider),
        LibraryRowDensity.compact,
      );
      expect(
        tester.getSize(rowByText('Demo Track')).height,
        LibraryRowDensity.compact.height,
      );

      await tester.tap(find.bySemanticsLabel('Toggle row layout'));
      await tester.pumpAndSettle();
      expect(
        container.read(libraryRowDensityProvider),
        LibraryRowDensity.comfortable,
      );
    },
  );

  testWidgets(
    'the density button is session-local and does not write settings',
    (tester) async {
      final container = await pumpList(tester);
      final saved = container.read(appSettingsProvider).asData!.value;

      await tester.tap(find.bySemanticsLabel('Toggle row layout'));
      await tester.pumpAndSettle();

      // The override is live, but the persisted setting is untouched.
      expect(
        container.read(libraryRowDensityOverrideProvider),
        LibraryRowDensity.compact,
      );
      expect(
        container.read(libraryRowDensitySettingProvider),
        LibraryRowDensity.comfortable,
      );
      expect(
        container.read(appSettingsProvider).asData!.value.libraryRowDensity,
        saved.libraryRowDensity,
      );
    },
  );

  testWidgets('the two layouts show the same track metadata', (tester) async {
    // BPM and key sit in the trailing meta in BOTH densities, and carry the
    // same " BPM" suffix, so these assertions hold either way round.
    for (final density in LibraryRowDensity.values) {
      final container = await pumpList(tester);
      container.read(libraryRowDensityOverrideProvider.notifier).set(density);
      await tester.pumpAndSettle();

      expect(find.text('Demo Track'), findsOneWidget, reason: density.name);
      expect(find.text('128.0 BPM'), findsOneWidget, reason: density.name);
      expect(
        find.text(formatDeckKey(track.key, KeyDisplayMode.musical)),
        findsOneWidget,
        reason: density.name,
      );
    }
  });

  testWidgets(
    'comfortable pills its metadata and keeps the artist by the title',
    (tester) async {
      await pumpList(
        tester,
        width: 1000,
        tracks: const [
          LibraryTrackSummary(
            id: 't1',
            displayName: 'Demo Track',
            title: 'Demo Track',
            artist: 'Artist',
            album: 'Album',
            genre: 'Genre',
            bpm: 128,
            key: '8A',
            durationMs: 180000,
            path: '/tmp/samples/demo.wav',
          ),
        ],
      );

      final pills = find
          .descendant(
            of: find.byType(TrackListRow),
            matching: find.byType(Wrap),
          )
          .first;
      for (final label in ['Album', 'Genre', '3:00']) {
        expect(
          find.descendant(of: pills, matching: find.text(label)),
          findsOneWidget,
          reason: 'missing pill "$label"',
        );
      }

      // The artist is not a pill: it rides the title line after a separator. A
      // baseline cannot be read back after layout (`getDistanceToBaseline` is
      // layout-time only), so the shared baseline is asserted where the line
      // declares it.
      final title = tester.getCenter(find.text('Demo Track'));
      final artist = tester.getCenter(find.text('Artist'));
      expect(artist.dx, greaterThan(title.dx));
      final bullet = tester.getCenter(find.text('•'));
      expect(bullet.dx, greaterThan(title.dx));
      expect(bullet.dx, lessThan(artist.dx));

      final line = tester.widget<Row>(
        find
            .descendant(
              of: find.byType(TrackTitleLine),
              matching: find.byType(Row),
            )
            .first,
      );
      expect(line.crossAxisAlignment, CrossAxisAlignment.baseline);
      expect(line.textBaseline, TextBaseline.alphabetic);

      // BPM and key are pills too, but grouped on the right rather than in the
      // metadata Wrap, so scope to that group.
      final trailing = find.byKey(kTrailingMetaKey);
      expect(
        find.descendant(of: trailing, matching: find.text('128.0 BPM')),
        findsOneWidget,
      );
    },
  );

  testWidgets('compact keeps the artist on the title line', (tester) async {
    final container = await pumpList(tester, width: 1000);
    container
        .read(libraryRowDensityOverrideProvider.notifier)
        .set(LibraryRowDensity.compact);
    await tester.pumpAndSettle();

    // Both densities name the track the same way: title • artist.
    expect(find.text('Artist'), findsOneWidget);
    expect(find.text('•'), findsOneWidget);
    expect(
      tester.getCenter(find.text('Artist')).dx,
      greaterThan(tester.getCenter(find.text('Demo Track')).dx),
    );
  });

  testWidgets('metadata pills hang left to align with the title', (
    tester,
  ) async {
    await pumpList(
      tester,
      width: 1000,
      tracks: const [
        LibraryTrackSummary(
          id: 't1',
          displayName: 'Demo Track',
          title: 'Demo Track',
          artist: 'Artist',
          album: 'Album',
          bpm: 128,
          key: '8A',
          durationMs: 180000,
          path: '/tmp/samples/demo.wav',
        ),
      ],
    );

    // `muted` matches the list surface, so the pill's horizontal padding has no
    // visible chip to justify it — the group is shifted back so the metadata
    // text lines up with the title above it.
    final titleLeft = tester.getTopLeft(find.text('Demo Track')).dx;
    final albumLeft = tester.getTopLeft(find.text('Album')).dx;
    expect(albumLeft, closeTo(titleLeft, 0.5));
  });

  testWidgets('whatever metadata survives stays inside the row', (
    tester,
  ) async {
    // Extras that cannot pack into the row's two runs are dropped; the rest must
    // stay inside the row's fixed height instead of being clipped out of it.
    final overflows = <String>[];
    final previous = FlutterError.onError;
    FlutterError.onError = (d) {
      final text = d.toString();
      if (text.contains('overflowed')) overflows.add(text);
      previous?.call(d);
    };
    try {
      const labels = [
        'A Really Very Extremely Long Artist Name Indeed',
        'A Long Album Title',
        'Drum And Bass',
        '3:00',
      ];
      for (final width in [320.0, 420.0, 700.0, 1000.0]) {
        await pumpList(
          tester,
          width: width,
          tracks: const [
            LibraryTrackSummary(
              id: 't1',
              displayName: 'Demo Track',
              title: 'Demo Track',
              artist: 'A Really Very Extremely Long Artist Name Indeed',
              album: 'A Long Album Title',
              genre: 'Drum And Bass',
              durationMs: 180000,
              path: '/tmp/samples/demo.wav',
            ),
          ],
        );

        final row = tester.getRect(find.byType(TrackListRow));
        expect(
          find.text('3:00'),
          findsOneWidget,
          reason: 'duration missing at $width',
        );
        if (width == 1000) {
          // Room to spare: every label is kept, so the wide case still pins the
          // two-run layout.
          for (final label in labels) {
            expect(
              find.text(label),
              findsOneWidget,
              reason: '$label missing at $width',
            );
          }
        }
        for (final label in labels) {
          final shown = find.text(label);
          if (shown.evaluate().isEmpty) {
            continue; // Dropped on purpose: it could not pack.
          }
          expect(
            tester.getRect(shown).bottom,
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

  testWidgets('BPM and key are pills carrying their glyphs', (tester) async {
    for (final density in LibraryRowDensity.values) {
      final container = await pumpList(tester);
      container.read(libraryRowDensityOverrideProvider.notifier).set(density);
      await tester.pumpAndSettle();

      expect(
        find.descendant(
          of: find.byKey(kTrailingMetaKey),
          matching: find.byIcon(LucideIcons.metronome),
        ),
        findsOneWidget,
        reason: '${density.name} is missing the BPM glyph',
      );
      expect(
        find.descendant(
          of: find.byKey(kTrailingMetaKey),
          matching: find.byIcon(LucideIcons.music2),
        ),
        findsOneWidget,
        reason: '${density.name} is missing the key glyph',
      );
    }
  });

  testWidgets('BPM and key render as pills, not bare text', (tester) async {
    await pumpList(tester, width: 1000);

    final trailing = find.byKey(kTrailingMetaKey);
    for (final label in [
      '128.0 BPM',
      formatDeckKey(track.key, KeyDisplayMode.musical),
    ]) {
      final text = find.descendant(of: trailing, matching: find.text(label));
      expect(
        text,
        findsOneWidget,
        reason: '"$label" missing from trailing meta',
      );

      // A pill is a rounded, filled box between the label and the group. This
      // is what distinguishes a chip from the plain trailing text it replaced.
      expect(
        find.ancestor(
          of: text,
          matching: find.byWidgetPredicate(
            (w) =>
                w is DecoratedBox &&
                w.decoration is BoxDecoration &&
                (w.decoration as BoxDecoration).borderRadius != null,
          ),
        ),
        findsWidgets,
        reason: '"$label" is not inside a pill',
      );
    }
  });

  testWidgets('trailing pills are never ellipsized at workable widths', (
    tester,
  ) async {
    // The slot widths are measured constants; if the font or the label format
    // changes, the trailing pills would silently truncate to "109.7 B...".
    // Compact reserves an extra duration slot, so its floor is higher — below
    // roughly 480px pane width it squeezes by design.
    for (final (density, widths) in [
      (LibraryRowDensity.comfortable, [420.0, 700.0, 1000.0, 1900.0]),
      (LibraryRowDensity.compact, [520.0, 1000.0, 1900.0]),
    ]) {
      for (final width in widths) {
        final container = await pumpList(tester, width: width);
        container.read(libraryRowDensityOverrideProvider.notifier).set(density);
        await tester.pumpAndSettle();

        for (final label in [
          '128.0 BPM',
          formatDeckKey(track.key, KeyDisplayMode.musical),
        ]) {
          final text = find.descendant(
            of: find.byKey(kTrailingMetaKey),
            matching: find.text(label),
          );
          expect(
            text,
            findsOneWidget,
            reason: '"$label" missing in ${density.name} at $width',
          );
          expect(
            tester.renderObject<RenderParagraph>(text).didExceedMaxLines,
            isFalse,
            reason: '"$label" was truncated in ${density.name} at $width',
          );
        }
      }
    }
  });

  testWidgets('title and pill stack are vertically centred in the row', (
    tester,
  ) async {
    // Measure the pill *chip*, not its container: a fixed-height pill area
    // leaves the chip at the top of an empty box, which reads as the title
    // being pushed to the top even though the container itself is centred.
    for (final trackInTest in const [
      LibraryTrackSummary(
        id: 't1',
        displayName: 'With Artist',
        title: 'With Artist',
        artist: 'Artist',
        durationMs: 180000,
        path: '/tmp/a.wav',
      ),
      LibraryTrackSummary(
        id: 't2',
        displayName: 'No Pills At All',
        title: 'No Pills At All',
        path: '/tmp/b.wav',
      ),
    ]) {
      await pumpList(tester, tracks: [trackInTest]);

      final row = tester.getRect(find.byType(TrackListRow));
      final title = tester.getRect(find.text(trackInTest.displayName));
      final chips = find
          .descendant(
            of: find.byType(TrackListRow),
            matching: find.byWidgetPredicate((w) {
              if (w is! DecoratedBox) return false;
              final d = w.decoration;
              if (d is! BoxDecoration || d.borderRadius == null) return false;
              // Filled only: the actions button is a rounded box too, but its
              // fill is transparent until hovered.
              return (d.color?.a ?? 0) > 0;
            }),
          )
          .evaluate();
      // The lowest chip edge: the bottom of the visible stack.
      final contentBottom = chips.isEmpty
          ? title.bottom
          : chips
                .map((e) => tester.getRect(find.byWidget(e.widget)).bottom)
                .reduce((a, b) => a > b ? a : b);

      final above = title.top - row.top;
      final below = row.bottom - contentBottom;
      expect(
        above,
        closeTo(below, 1.0),
        reason: '${trackInTest.displayName}: $above px above, $below px below',
      );
    }
  });

  testWidgets('clearing the density override falls back to the saved default', (
    tester,
  ) async {
    // `saveAppSettings` invalidates the override so a freshly saved density
    // takes effect. This pins the precedence that makes that necessary: the
    // session override outranks the saved setting.
    final container = await pumpList(tester);
    expect(
      container.read(libraryRowDensityProvider),
      LibraryRowDensity.comfortable,
    );

    container
        .read(libraryRowDensityOverrideProvider.notifier)
        .set(LibraryRowDensity.compact);
    expect(
      container.read(libraryRowDensityProvider),
      LibraryRowDensity.compact,
      reason: 'the override should outrank the saved default',
    );

    container.invalidate(libraryRowDensityOverrideProvider);
    expect(
      container.read(libraryRowDensityProvider),
      LibraryRowDensity.comfortable,
      reason: 'a cleared override should restore the saved default',
    );
  });

  testWidgets('compact keeps the length in its trailing group', (tester) async {
    // Compact has no metadata pills row, so without this the track length is
    // invisible in compact — a regression against the old grid's Length column.
    final container = await pumpList(tester);
    final trailing = find.byKey(kTrailingMetaKey);

    // Comfortable carries the duration on the left, so it must not duplicate it
    // into the trailing group.
    expect(
      find.descendant(of: trailing, matching: find.text('3:00')),
      findsNothing,
      reason: 'comfortable duplicated the duration in the trailing group',
    );

    container
        .read(libraryRowDensityOverrideProvider.notifier)
        .set(LibraryRowDensity.compact);
    await tester.pumpAndSettle();

    expect(
      find.descendant(of: trailing, matching: find.text('3:00')),
      findsOneWidget,
      reason: 'compact lost the track length',
    );
    // Order matters: BPM and key stay flush right in both densities.
    final order = find
        .descendant(of: trailing, matching: find.byType(Text))
        .evaluate()
        .map((e) => (e.widget as Text).data)
        .toList();
    expect(order, [
      '3:00',
      '128.0 BPM',
      formatDeckKey(track.key, KeyDisplayMode.musical),
    ]);
  });

  group('fitsInRuns', () {
    test('packs greedily, and an over-wide item takes its own run', () {
      expect(fitsInRuns([60, 60, 60], available: 130, runs: 2), isTrue);
      expect(fitsInRuns([120, 120, 120], available: 130, runs: 2), isFalse);
      expect(fitsInRuns([200, 40], available: 130, runs: 2), isTrue);
      expect(fitsInRuns([200, 40], available: 130, runs: 1), isFalse);
      expect(fitsInRuns(const [10], available: 0, runs: 2), isFalse);
    });
  });

  testWidgets('a title that needs the line hides the artist', (tester) async {
    await pumpList(
      tester,
      width: 360,
      tracks: const [
        LibraryTrackSummary(
          id: 't1',
          displayName: 'Long',
          title: 'A Really Very Extremely Long Track Title Indeed',
          artist: 'Artist',
          bpm: 128,
          key: '8A',
          durationMs: 180000,
          path: '/tmp/samples/demo.wav',
        ),
      ],
    );

    // The title keeps its whole line: the artist and its bullet give way rather
    // than squeezing the title into an ellipsis.
    expect(
      find.text('A Really Very Extremely Long Track Title Indeed'),
      findsOneWidget,
    );
    expect(find.text('Artist'), findsNothing);
    expect(find.text('•'), findsNothing);
  });

  testWidgets('album and genre go when the metadata cannot pack', (
    tester,
  ) async {
    await pumpList(
      tester,
      width: 420,
      tracks: const [
        LibraryTrackSummary(
          id: 't1',
          displayName: 'Demo Track',
          title: 'Demo Track',
          artist: 'Artist',
          album: 'A Very Long Album Title That Keeps Going',
          genre: 'Progressive Melodic House Music',
          bpm: 128,
          key: '8A',
          durationMs: 180000,
          path: '/tmp/samples/demo.wav',
        ),
      ],
    );

    // Three runs at their natural width: the two extras go together and the
    // duration stays, instead of a cramped extra run being clipped.
    expect(find.text('A Very Long Album Title That Keeps Going'), findsNothing);
    expect(find.text('Progressive Melodic House Music'), findsNothing);
    expect(find.text('3:00'), findsOneWidget);
  });

  testWidgets('anything shown paints whole, never ellipsized', (tester) async {
    // The rule is hide-before-clip, not clip-and-hide. Whatever the row decides
    // to keep must paint at its natural width: an artist beside an ellipsized
    // title, or a pill clipped at some arbitrary cap, is the bug.
    for (final width in [380.0, 476.0, 700.0, 1000.0]) {
      await pumpList(
        tester,
        width: width,
        tracks: const [
          LibraryTrackSummary(
            id: 't1',
            displayName: 'Anade Acuite',
            title: 'Anade Acuite',
            artist: 'Hercules',
            album: 'Reggaeton 2025',
            genre: 'Reggaeton',
            bpm: 96,
            key: 'Am',
            durationMs: 226000,
            path: '/tmp/samples/demo.wav',
          ),
        ],
      );

      final context = tester.element(find.byType(TrackTitleLine));
      final typography = MixarTheme.of(context).typography;
      final titleStyle = typography.body.sm.copyWith(
        fontWeight: FontWeight.w600,
      );
      final pillStyle = typography.body.xs.copyWith(
        fontWeight: FontWeight.w500,
      );

      double paintedWidth(String label) =>
          tester.getSize(find.text(label)).width;

      final artist = find.text('Hercules');
      if (artist.evaluate().isNotEmpty) {
        expect(
          paintedWidth('Anade Acuite'),
          greaterThanOrEqualTo(
            measureTextWidth(context, 'Anade Acuite', titleStyle) - 0.5,
          ),
          reason: 'the artist is shown while the title ellipsized at $width',
        );
      }

      final column = tester.getSize(find.byType(TrackTitleLine)).width;
      for (final label in ['Reggaeton 2025', 'Reggaeton', '3:46']) {
        final pill = find.text(label);
        if (pill.evaluate().isEmpty) {
          continue;
        }
        final natural = measureTextWidth(context, label, pillStyle);
        if (natural + kMetaPillPadding.horizontal > column) {
          // No run is wide enough anywhere, so even a mandatory pill (the
          // duration) has to clip. Nothing to decide.
          continue;
        }
        // The label inside a pill: padding and the clock glyph sit outside it.
        expect(
          paintedWidth(label),
          greaterThanOrEqualTo(natural - 0.5),
          reason: '$label is shown clipped at $width',
        );
      }
    }
  });

  testWidgets('compact hides the pills', (tester) async {
    final container = await pumpList(tester);
    container
        .read(libraryRowDensityOverrideProvider.notifier)
        .set(LibraryRowDensity.compact);
    await tester.pumpAndSettle();

    expect(find.byType(Wrap), findsNothing);
    // Compact names the track — title • artist — but keeps the metadata pills
    // off; the trailing meta carries BPM and key.
    expect(find.text('Artist'), findsOneWidget);
    expect(find.text('•'), findsOneWidget);
    // The trailing meta still carries the values compact can show.
    expect(find.text('128.0 BPM'), findsOneWidget);
  });

  testWidgets('every sort menu row is the same single-line height', (
    tester,
  ) async {
    await pumpList(tester);
    await tester.tap(find.bySemanticsLabel(RegExp('Sort tracks')));
    await tester.pumpAndSettle();

    final items = find.byType(MixarMenuItem);
    // One row per field — direction lives on the active row, not a second group.
    expect(items, findsNWidgets(LibrarySortField.values.length));
    final heights = <double>[];
    for (var i = 0; i < items.evaluate().length; i++) {
      heights.add(tester.getSize(items.at(i)).height);
    }
    // A label wide enough to wrap doubles its own row and leaves the menu with
    // mixed row heights, which is what this guards against.
    expect(heights.toSet(), hasLength(1), reason: 'heights: $heights');
    for (var i = 0; i < items.evaluate().length; i++) {
      final label = find
          .descendant(of: items.at(i), matching: find.byType(Text))
          .first;
      expect(
        tester.renderObject<RenderParagraph>(label).size.height,
        lessThan(30),
        reason: 'sort row $i wrapped to two lines',
      );
    }
  });

  testWidgets('the toolbar does not overflow a narrow library pane', (
    tester,
  ) async {
    final overflows = <String>[];
    final previous = FlutterError.onError;
    FlutterError.onError = (d) {
      final text = d.toString();
      if (text.contains('overflowed')) overflows.add(text);
      previous?.call(d);
    };
    try {
      await pumpList(tester, width: 260);
    } finally {
      FlutterError.onError = previous;
    }
    expect(overflows, isEmpty, reason: 'toolbar overflowed: $overflows');
    // Both controls survive a squeeze: the density toggle is the last child and
    // would be pushed off the end by a rigid sort label.
    expect(find.bySemanticsLabel('Toggle row layout'), findsOneWidget);
  });

  testWidgets('the sort button hugs its label instead of filling the toolbar', (
    tester,
  ) async {
    await pumpList(tester, width: 1600);

    final sortButton = find.bySemanticsLabel(RegExp('Sort tracks'));
    final button = tester.getSize(sortButton);
    // Content-sized: "Title" (60) + gap + arrow (12) + button padding.
    // Stretching to the 120px cap — via a flex child, or via AppButton's
    // default MainAxisSize.max — is what this guards against.
    expect(button.width, lessThan(100), reason: 'width was ${button.width}');
    // It is still right-aligned next to the density toggle, not pushed around.
    expect(find.bySemanticsLabel('Toggle row layout'), findsOneWidget);
  });

  testWidgets('the artwork thumb is square in both densities', (tester) async {
    // Set the density explicitly rather than toggling: toggling leaves the
    // next iteration reading whatever the previous one produced.
    for (final density in LibraryRowDensity.values) {
      final container = await pumpList(tester);
      container.read(libraryRowDensityOverrideProvider.notifier).set(density);
      await tester.pumpAndSettle();

      expect(container.read(libraryRowDensityProvider), density);

      // Placeholder state: no artwork is cached, so the thumb is the muted box.
      final thumb = find
          .descendant(
            of: find.byType(TrackListRow),
            matching: find.byType(ColoredBox),
          )
          .first;
      final size = tester.getSize(thumb);
      expect(
        size,
        Size.square(density.artSize),
        reason: '${density.name} thumb is ${size.width}x${size.height}',
      );
      // Square means square: an unconstrained placeholder collapses to the
      // icon's width while the row stretches it to the full row height.
      expect(size.width, size.height);
      expect(size.height, lessThanOrEqualTo(density.height));
    }
  });

  /// Resolved style of the key label, in whichever density is showing.
  ///
  /// Both densities render the key through the same trailing pill — a plain
  /// `Text` inside `_MetaPill` — so there is a single shape to match and no
  /// rich-text variant to fall back to.
  TextStyle keyStyleInPane(WidgetTester tester) {
    final text = find.descendant(
      of: find.byKey(kTrailingMetaKey),
      matching: find.text('8A'),
    );
    expect(text, findsOneWidget, reason: 'no key label in the trailing meta');
    final style = tester.widget<Text>(text).style;
    expect(style, isNotNull, reason: 'key label has no explicit style');
    return style!;
  }

  Future<void> pumpKeyedPane(
    WidgetTester tester,
    KeyColorModeSetting mode,
  ) async {
    debugOverrideDesktopWindow = false;
    addTearDown(() => debugOverrideDesktopWindow = null);
    final theme = MixarThemeData.light();
    tester.view.physicalSize = const Size(1200, 600);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);

    await tester.pumpWidget(
      ProviderScope(
        // Keyed by mode: re-pumping a ProviderScope otherwise reuses its
        // container, so a second pump in the same test would keep the first
        // pump's settings override (and its session-only density override).
        key: ValueKey(mode),
        overrides: [
          collectionsProvider.overrideWith((ref) async => [collection]),
          overrideCollectionTracksValue([keyedTrack]),
          libraryEventsBootstrapProvider.overrideWith((ref) {}),
          appSettingsProvider.overrideWith(
            (ref) async => copyAppSettings(
              defaultAppSettings(),
              keyColorMode: mode,
              keyDisplayMode: KeyDisplayModeSetting.camelot,
            ),
          ),
        ],
        child: MaterialApp(
          theme: materialUiThemeFromMixar(theme),
          builder: mixarMaterialAppBuilder(theme),
          home: const Scaffold(
            body: SizedBox(width: 1200, height: 600, child: TrackListPane()),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();
  }

  // One test per mode: re-pumping a ProviderScope reuses its container, so a
  // loop would not pick up a new settings override.
  for (final mode in KeyColorModeSetting.values) {
    testWidgets(
      'both densities render the ${mode.name} key colour identically',
      (tester) async {
        await pumpKeyedPane(tester, mode);
        final comfortable = keyStyleInPane(tester);

        await tester.tap(find.bySemanticsLabel('Toggle row layout'));
        await tester.pumpAndSettle();
        final compact = keyStyleInPane(tester);

        expect(
          compact.color,
          comfortable.color,
          reason: 'compact and comfortable disagree on key colour',
        );
        expect(
          compact.fontWeight,
          comfortable.fontWeight,
          reason: 'compact and comfortable disagree on key weight',
        );
      },
    );
  }

  testWidgets('the key colour setting actually reaches the row', (
    tester,
  ) async {
    await pumpKeyedPane(tester, KeyColorModeSetting.off);
    final off = keyStyleInPane(tester).color;

    await pumpKeyedPane(tester, KeyColorModeSetting.absolute);
    final absolute = keyStyleInPane(tester).color;

    expect(absolute, isNot(off));
  });

  test('formatTrackDuration separates unknown from a real short length', () {
    // Unknown metadata renders nothing, so no chip appears. A *known* length
    // must not be mistaken for that, even below one second.
    for (final unknown in [null, 0, -1]) {
      expect(formatTrackDuration(unknown), '', reason: 'unknown: $unknown');
    }
    expect(formatTrackDuration(400), '0:00');
    expect(formatTrackDuration(1000), '0:01');
    // Floored, matching the pre-refactor table and how file managers show it.
    expect(formatTrackDuration(59_999), '0:59');
    expect(formatTrackDuration(60_000), '1:00');
    expect(formatTrackDuration(3_661_000), '61:01');
  });

  test('LibrarySort.select starts new fields ascending and toggles active', () {
    final container = ProviderContainer();
    addTearDown(container.dispose);

    // Pin the default so a changed default cannot silently skip the
    // "new field starts ascending" path exercised below.
    expect(container.read(librarySortProvider), (LibrarySortField.title, true));

    // A new field always starts ascending even if the previous field was
    // flipped to descending.
    container.read(librarySortProvider.notifier).select(LibrarySortField.bpm);
    expect(container.read(librarySortProvider), (LibrarySortField.bpm, true));
    container.read(librarySortProvider.notifier).select(LibrarySortField.bpm);
    expect(container.read(librarySortProvider), (LibrarySortField.bpm, false));
    container
        .read(librarySortProvider.notifier)
        .select(LibrarySortField.artist);
    expect(container.read(librarySortProvider), (
      LibrarySortField.artist,
      true,
    ));
  });

  test('sorting is stable, case-insensitive, and puts nulls last', () {
    LibraryTrackSummary t(String id, {String? title, double? bpm}) =>
        LibraryTrackSummary(
          id: id,
          displayName: id,
          title: title,
          bpm: bpm,
          path: '/tmp/$id.wav',
        );
    final tracks = [
      t('a', title: 'banana'),
      t('b', title: 'Apple'),
      t('c', bpm: 128),
      t('d', bpm: 90),
    ];

    List<String> ids(LibrarySortField field, {bool ascending = true}) => [
      for (final track in sortLibraryTracks(
        tracks,
        field,
        ascending: ascending,
      ))
        track.id,
    ];

    // Case-insensitive: 'Apple' sorts before 'banana', then the untitled
    // tracks fall back to their display name.
    expect(ids(LibrarySortField.title), ['b', 'a', 'c', 'd']);
    // Descending reverses the known values; the two unknown-BPM tracks stay at
    // the bottom rather than floating to the top.
    expect(ids(LibrarySortField.title, ascending: false), ['d', 'c', 'a', 'b']);
    // Numeric, not lexical: 90 before 128 ascending, 128 first descending.
    expect(ids(LibrarySortField.bpm), ['d', 'c', 'a', 'b']);
    expect(ids(LibrarySortField.bpm, ascending: false), ['c', 'd', 'a', 'b']);
  });

  test('a non-finite or negative sort value is treated as missing', () {
    final tracks = [
      const LibraryTrackSummary(
        id: 'good',
        displayName: 'good',
        bpm: 128,
        path: '/tmp/good.wav',
      ),
      const LibraryTrackSummary(
        id: 'nan',
        displayName: 'nan',
        bpm: double.nan,
        path: '/tmp/nan.wav',
      ),
      const LibraryTrackSummary(
        id: 'inf',
        displayName: 'inf',
        bpm: double.infinity,
        path: '/tmp/inf.wav',
      ),
    ];
    List<String> ids({required bool ascending}) => [
      for (final track in sortLibraryTracks(
        tracks,
        LibrarySortField.bpm,
        ascending: ascending,
      ))
        track.id,
    ];
    expect(ids(ascending: true), ['good', 'nan', 'inf']);
    expect(ids(ascending: false), ['good', 'nan', 'inf']);
  });

  test(
    'sorting keeps provider order for ties past the stable-sort threshold',
    () {
      // Dart's List.sort is only stable below its insertion-sort threshold (~32
      // elements), so a comparator that returns 0 for equal keys lets a real
      // library shuffle tied rows. Every track here ties on BPM, so the output
      // must still be exactly provider order.
      final tracks = [
        for (var i = 0; i < 200; i++)
          LibraryTrackSummary(
            id: 't$i',
            displayName: 't$i',
            path: '/tmp/$i.wav',
          ),
      ];

      for (final ascending in [true, false]) {
        final sorted = sortLibraryTracks(
          tracks,
          LibrarySortField.bpm,
          ascending: ascending,
        );
        expect(
          sorted.map((t) => t.id),
          tracks.map((t) => t.id),
          reason: 'ties reordered (ascending: $ascending)',
        );
      }
    },
  );

  test('BPM sorting distinguishes values inside the same whole number', () {
    // 109.7 and 109.9 both round to 110, so a whole-unit sort key makes them a
    // tie and leaves them in provider order. Deliberately listed descending so
    // a lossy key cannot pass by accident.
    final tracks = [
      const LibraryTrackSummary(
        id: 'high',
        displayName: 'high',
        bpm: 109.9,
        path: '/tmp/high.wav',
      ),
      const LibraryTrackSummary(
        id: 'low',
        displayName: 'low',
        bpm: 109.7,
        path: '/tmp/low.wav',
      ),
    ];

    expect(
      sortLibraryTracks(
        tracks,
        LibrarySortField.bpm,
        ascending: true,
      ).map((t) => t.id),
      ['low', 'high'],
    );
    expect(
      sortLibraryTracks(
        tracks,
        LibrarySortField.bpm,
        ascending: false,
      ).map((t) => t.id),
      ['high', 'low'],
    );
  });

  test('every sort field orders known values and sinks missing ones', () {
    LibraryTrackSummary t(
      String id, {
      String? artist,
      String? key,
      int? durationMs,
    }) => LibraryTrackSummary(
      id: id,
      displayName: id,
      artist: artist,
      key: key,
      durationMs: durationMs,
      path: '/tmp/$id.wav',
    );

    final tracks = [
      t('alpha', artist: 'Alpha', key: '8A', durationMs: 60_000),
      t('beta', artist: 'beta', key: '3B', durationMs: 180_000),
      t('empty'),
    ];

    List<String> ids(LibrarySortField field, {bool ascending = true}) => [
      for (final track in sortLibraryTracks(
        tracks,
        field,
        ascending: ascending,
      ))
        track.id,
    ];

    // Case-insensitive text, then the track with no value last.
    expect(ids(LibrarySortField.artist), ['alpha', 'beta', 'empty']);
    expect(ids(LibrarySortField.artist, ascending: false), [
      'beta',
      'alpha',
      'empty',
    ]);
    // Keys sort by Camelot wheel position, not by label.
    expect(ids(LibrarySortField.key), ['beta', 'alpha', 'empty']);
    expect(ids(LibrarySortField.key, ascending: false), [
      'alpha',
      'beta',
      'empty',
    ]);
    // Durations order numerically, not lexically: 60s before 180s.
    expect(ids(LibrarySortField.length), ['alpha', 'beta', 'empty']);
    expect(ids(LibrarySortField.length, ascending: false), [
      'beta',
      'alpha',
      'empty',
    ]);
  });

  test('an out-of-range duration cannot invert the numeric key order', () {
    // Keys are scaled to milli-units and padded to a fixed width. A value past
    // the clamp scales to more than that width, `padLeft` stops aligning, and
    // '1000…' then compares below '999…' — silently sorting a shorter track
    // after a longer one. Listed ascending so the inversion is detectable.
    LibraryTrackSummary t(String id, {int? durationMs}) => LibraryTrackSummary(
      id: id,
      displayName: id,
      durationMs: durationMs,
      path: '/tmp/$id.wav',
    );

    final tracks = [
      // Just over the clamp: 31.7 years in milliseconds.
      t('beyond', durationMs: 1_000_000_000_000),
      // Just under it, which seeds the 15-digit key that used to win.
      t('below', durationMs: 999_999_999_999),
      t('int64max', durationMs: 9223372036854775807),
      t('typical', durationMs: 240_000),
      t('unknown'),
    ];

    expect(
      sortLibraryTracks(
        tracks,
        LibrarySortField.length,
        ascending: true,
      ).map((t) => t.id),
      ['typical', 'below', 'beyond', 'int64max', 'unknown'],
    );
    // Both out-of-range values clamp to the same key, so they tie and keep
    // provider order; only the values below the clamp are truly reversed.
    expect(
      sortLibraryTracks(
        tracks,
        LibrarySortField.length,
        ascending: false,
      ).map((t) => t.id),
      ['beyond', 'int64max', 'below', 'typical', 'unknown'],
    );
  });

  test('key sorting follows the Camelot wheel, not the label', () {
    LibraryTrackSummary t(String id, String? key) => LibraryTrackSummary(
      id: id,
      displayName: id,
      key: key,
      path: '/tmp/$id.wav',
    );

    // Labels sort as text as '10A', '12B', '1B', '2A' — four groups in an
    // order no DJ reads as meaningful.
    final tracks = [
      t('10a', '10A'),
      t('2a', '2A'),
      t('1b', '1B'),
      t('12b', '12B'),
      t('none', null),
    ];
    List<String> ids({required bool ascending}) => [
      for (final track in sortLibraryTracks(
        tracks,
        LibrarySortField.key,
        ascending: ascending,
      ))
        track.id,
    ];

    expect(ids(ascending: true), ['1b', '2a', '10a', '12b', 'none']);
    expect(ids(ascending: false), ['12b', '10a', '2a', '1b', 'none']);

    // Either key format maps to the same slot, so a musical "Am" ties with its
    // Camelot equivalent "8A" rather than sorting away from it.
    final mixed = [t('musical', 'Am'), t('camelot', '8A')];
    expect(
      sortLibraryTracks(
        mixed,
        LibrarySortField.key,
        ascending: true,
      ).map((t) => t.id),
      ['musical', 'camelot'],
      reason: 'Am and 8A are the same slot and should tie',
    );
  });

  test('sorting does not mutate the provider list', () {
    final tracks = [
      const LibraryTrackSummary(
        id: 'b',
        displayName: 'B',
        title: 'B',
        path: '/tmp/b.wav',
      ),
      const LibraryTrackSummary(
        id: 'a',
        displayName: 'A',
        title: 'A',
        path: '/tmp/a.wav',
      ),
    ];
    final sorted = sortLibraryTracks(
      tracks,
      LibrarySortField.title,
      ascending: true,
    );
    expect(sorted.map((t) => t.id), ['a', 'b']);
    expect(tracks.map((t) => t.id), ['b', 'a']);
  });

  testWidgets('the list surface is flush and borderless', (tester) async {
    // The list fills its pane edge to edge: removing the card chrome means no
    // border or radius insets it from the panel edges, so the row fill runs
    // right to the sides.
    await pumpList(tester, tracks: const [track, trackB]);

    final listView = find.byType(ListView);
    final pane = tester.getRect(find.byType(TrackListPane));
    final list = tester.getRect(listView);
    expect(list.left, pane.left, reason: 'list is not flush to the left edge');
    expect(
      list.right,
      pane.right,
      reason: 'list is not flush to the right edge',
    );

    // No bordered or rounded surface wraps the rows. Toolbar controls carry
    // borders too, so match on wrapping the list rather than on the pane.
    final surfaces = find
        .descendant(
          of: find.byType(TrackListPane),
          matching: find.byWidgetPredicate(
            (w) =>
                w is DecoratedBox &&
                w.decoration is BoxDecoration &&
                ((w.decoration as BoxDecoration).border != null ||
                    (w.decoration as BoxDecoration).borderRadius != null),
          ),
        )
        .evaluate();
    final wrapping = [
      for (final element in surfaces)
        if (find
            .descendant(of: find.byWidget(element.widget), matching: listView)
            .evaluate()
            .isNotEmpty)
          element.widget,
    ];

    expect(
      wrapping,
      isEmpty,
      reason:
          'expected the list to have no bordered/rounded surface, found '
          '${wrapping.length}',
    );
  });
}
