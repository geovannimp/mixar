import 'package:flutter/gestures.dart';

import 'package:flutter/rendering.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/library/providers.dart';
import 'package:gui_flutter/library/track_list_pane.dart';
import 'package:gui_flutter/mixer/engine_providers.dart';
import 'package:gui_flutter/mixer/key_format.dart';
import 'package:gui_flutter/settings/settings_defaults.dart';
import 'package:gui_flutter/settings/settings_providers.dart';
import 'package:gui_flutter/shell/desktop.dart';
import 'package:gui_flutter/shell/material_theme.dart';
import 'package:gui_flutter/shell/mixar_menu.dart';
import 'package:gui_flutter/shell/mixar_theme.dart';
import 'package:gui_flutter/src/rust/api/library.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';
import 'package:material_ui/material_ui.dart';
import 'package:super_drag_and_drop/super_drag_and_drop.dart';

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
          collectionTracksProvider.overrideWith((ref) async => tracks),
          libraryEventsBootstrapProvider.overrideWith((ref) {}),
          appSettingsProvider.overrideWith((ref) async => defaultAppSettings()),
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

  testWidgets('the sort menu reorders the list', (tester) async {
    final container = await pumpList(tester, tracks: const [track, trackB]);
    expect(
      container.read(libraryTableTracksProvider).asData!.value.first.id,
      't1',
    );

    await tester.tap(find.bySemanticsLabel(RegExp('Sort tracks')));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Sort by BPM'));
    await tester.pumpAndSettle();

    // 90 < 128, so the ascending default puts track B first.
    final sorted = container.read(libraryTableTracksProvider).asData!.value;
    expect(sorted.map((t) => t.id), ['t2', 't1']);
    expect(find.text('Other Track'), findsOneWidget);
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

  testWidgets('comfortable renders every metadata field as its own pill', (
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
          genre: 'Genre',
          bpm: 128,
          key: '8A',
          durationMs: 180000,
          path: '/tmp/samples/demo.wav',
        ),
      ],
    );

    final pills = find
        .descendant(of: find.byType(TrackListRow), matching: find.byType(Wrap))
        .first;
    for (final label in ['Artist', 'Album', 'Genre', '3:00']) {
      expect(
        find.descendant(of: pills, matching: find.text(label)),
        findsOneWidget,
        reason: 'missing pill "$label"',
      );
    }

    // BPM and key are pills too, but grouped on the right rather than in the
    // metadata Wrap, so scope to that group.
    final trailing = find.byKey(kTrailingMetaKey);
    expect(
      find.descendant(of: trailing, matching: find.text('128.0 BPM')),
      findsOneWidget,
    );
  });

  testWidgets('pills wrap to two runs and none is clipped', (tester) async {
    // A long artist name would otherwise take a whole run and push the rest
    // past the row's fixed height, where they would be silently clipped.
    final overflows = <String>[];
    final previous = FlutterError.onError;
    FlutterError.onError = (d) {
      final text = d.toString();
      if (text.contains('overflowed')) overflows.add(text);
      previous?.call(d);
    };
    try {
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
        for (final label in [
          'A Really Very Extremely Long Artist Name Indeed',
          'A Long Album Title',
          'Drum And Bass',
          '3:00',
        ]) {
          final pill = find.text(label);
          expect(pill, findsOneWidget, reason: '$label missing at $width');
          final r = tester.getRect(pill);
          expect(
            r.bottom,
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

      // The glyphs sit inside the trailing pill group.
      expect(
        find.descendant(
          of: find.byKey(kTrailingMetaKey),
          matching: find.byIcon(LucideIcons.metronome),
        ),
        findsOneWidget,
        reason: 'BPM glyph is not in the trailing pills',
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

  testWidgets('BPM and key labels are never ellipsized', (tester) async {
    // The slot widths are measured constants; if the font or the label format
    // changes, the trailing pills would silently truncate to "109.7 B...".
    for (final width in [420.0, 700.0, 1000.0, 1900.0]) {
      await pumpList(tester, width: width);
      for (final label in [
        '128.0 BPM',
        formatDeckKey(track.key, KeyDisplayMode.musical),
      ]) {
        final text = find.descendant(
          of: find.byKey(kTrailingMetaKey),
          matching: find.text(label),
        );
        expect(text, findsOneWidget, reason: '"$label" missing at $width');
        expect(
          tester.renderObject<RenderParagraph>(text).didExceedMaxLines,
          isFalse,
          reason: '"$label" was truncated at $width',
        );
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

  testWidgets('compact hides the pills', (tester) async {
    final container = await pumpList(tester);
    container
        .read(libraryRowDensityOverrideProvider.notifier)
        .set(LibraryRowDensity.compact);
    await tester.pumpAndSettle();

    expect(find.byType(Wrap), findsNothing);
    expect(find.text('Artist'), findsNothing);
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
    expect(items, findsNWidgets(6));
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

  /// Key label style in the current density.
  ///
  /// Compact renders the key as its own [Text]; comfortable renders it as a
  /// span inside the rich-text subtitle. `Text.rich` is itself a [Text] whose
  /// `data` is null, so match on `data` to tell the two apart.
  TextStyle keyStyleInPane(WidgetTester tester) {
    final row = find.byType(TrackListRow);
    for (final e
        in find.descendant(of: row, matching: find.byType(Text)).evaluate()) {
      final w = e.widget as Text;
      if (w.data == '8A' && w.style != null) {
        return w.style!;
      }
    }
    for (final e
        in find
            .descendant(of: row, matching: find.byType(RichText))
            .evaluate()) {
      final spans = <TextStyle>[];
      (e.widget as RichText).text.visitChildren((span) {
        if (span is TextSpan && span.text == '8A' && span.style != null) {
          spans.add(span.style!);
        }
        return true;
      });
      if (spans.isNotEmpty) {
        return spans.single;
      }
    }
    fail('no key label rendered inside the row');
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
          collectionTracksProvider.overrideWith((ref) async => [keyedTrack]),
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

  testWidgets('the list border paints above the rows, not behind them', (
    tester,
  ) async {
    // Rows fill `colors.secondary` edge to edge — the same colour as the
    // surface — so a border in a background decoration is painted over wherever
    // a row is, and only remains visible in the empty space below the last
    // row. Assert on the decoration layer, not on geometry.
    await pumpList(tester, tracks: const [track, trackB]);

    // The list surface is the bordered decoration that wraps the rows. Toolbar
    // controls carry background borders too, so match on that containment
    // rather than on the pane as a whole.
    final listView = find.byType(ListView);
    final surfaces = find
        .descendant(
          of: find.byType(TrackListPane),
          matching: find.byWidgetPredicate(
            (w) =>
                w is DecoratedBox &&
                w.decoration is BoxDecoration &&
                (w.decoration as BoxDecoration).border != null,
          ),
        )
        .evaluate();

    final wrapping = [
      for (final element in surfaces)
        if (find
            .descendant(of: find.byWidget(element.widget), matching: listView)
            .evaluate()
            .isNotEmpty)
          element.widget as DecoratedBox,
    ];

    expect(
      wrapping,
      hasLength(1),
      reason:
          'expected one bordered surface wrapping the list, got '
          '${wrapping.length}',
    );
    expect(
      wrapping.single.position,
      DecorationPosition.foreground,
      reason:
          'the border is painted behind the rows, so it is hidden wherever a '
          'row is drawn',
    );
  });
}
