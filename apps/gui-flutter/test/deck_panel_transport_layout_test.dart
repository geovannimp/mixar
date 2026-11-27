import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/models/engine_ui.dart';
import 'package:gui_flutter/core/theme/deck_accent.dart';
import 'package:gui_flutter/core/utils/desktop.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/providers/mixer/engine_providers.dart';
import 'package:gui_flutter/providers/settings/settings_providers.dart';
import 'package:gui_flutter/providers/shell/controller_providers.dart';
import 'package:gui_flutter/widgets/mixer/deck_cue_button.dart';
import 'package:gui_flutter/widgets/mixer/deck_panel.dart';
import 'package:gui_flutter/widgets/mixer/deck_performance_panel.dart';
import 'package:gui_flutter/widgets/mixer/deck_track_info.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';
import 'package:material_ui/material_ui.dart' show Scaffold;

import 'support/fake_engine_transport.dart';
import 'support/mixar_material_app.dart';

class _EngineUi extends EngineUi {
  new(this.snapshot);

  final EngineUiSnapshot snapshot;

  @override
  EngineUiSnapshot build() => snapshot;
}

/// Tolerance for sub-pixel layout rounding.
const _kEpsilon = 0.5;

/// Mirrors `_kDeckSideInset` in `deck_panel.dart`.
const _kSideInset = 10.0;

/// Mirrors `_kTransportRowGap` in `deck_panel.dart`.
const _kRowGap = 8.0;

void main() {
  Future<void> pumpDeck(
    WidgetTester tester, {
    required Size size,
    int deckId = 0,
    EngineUiSnapshot? seed,
  }) async {
    final snapshot =
        seed ?? const EngineUiSnapshot(running: true, trackPaths: {});
    debugOverrideDesktopWindow = false;
    addTearDown(() => debugOverrideDesktopWindow = null);
    tester.view.physicalSize = size;
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);

    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          engineUiProvider.overrideWith(() => _EngineUi(snapshot)),
          engineTransportProvider.overrideWith(
            (ref) async => FakeEngineTransport(),
          ),
          appSettingsProvider.overrideWith((ref) async => defaultAppSettings()),
          audioDevicesProvider.overrideWith((ref) async => const []),
          samplerBanksProvider.overrideWith((ref) async => const []),
          controllerTransportProvider.overrideWith((ref) async => null),
        ],
        child: mixarTestMaterialApp(
          theme: MixarThemeData.dark(),
          home: Scaffold(
            body: DeckPanel(
              deckId: deckId,
              label: 'DECK A',
              accent: DeckAccent.a,
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();
  }

  ({Rect panel, Rect cue, Rect play}) rects(WidgetTester tester) => (
    panel: tester.getRect(find.byType(DeckPerformancePanel)),
    cue: tester.getRect(find.byType(DeckCueButton)),
    play: tester.getRect(find.byIcon(LucideIcons.play)),
  );

  testWidgets('compact deck stacks circular transport beside the panel', (
    tester,
  ) async {
    await pumpDeck(tester, size: const Size(370, 290));

    final (:panel, :cue, :play) = rects(tester);
    final playTile = tester.getRect(
      find
          .ancestor(
            of: find.byIcon(LucideIcons.play),
            matching: find.byType(DecoratedBox),
          )
          .first,
    );

    // Beside the pads, not under them.
    expect(cue.left, greaterThanOrEqualTo(panel.right - _kEpsilon));
    // The panel runs flush to the deck's content edges, with no side margin.
    final board = tester.getRect(find.byType(DeckTrackInfo));
    expect(panel.left, board.left);
    // The pair keeps the deck's side gutter on the outside edge.
    expect(cue.right, board.right - _kSideInset);

    // The divider hugs the panel; the circles are centred in the gutter.
    final dividerFinder = find.byKey(
      const ValueKey('deck-performance-transport-divider'),
    );
    expect(dividerFinder, findsOneWidget);
    final divider = tester.getRect(dividerFinder);
    expect(
      divider.width,
      closeTo(MixarThemeData.dark().style.borderWidth, _kEpsilon),
    );
    expect(divider.left, closeTo(panel.right, _kEpsilon));
    expect(divider.center.dx, lessThan(cue.left));
    expect(cue.left - divider.right, closeTo(_kSideInset, _kEpsilon));
    final painted = tester.getRect(
      find
          .descendant(of: dividerFinder, matching: find.byType(ColoredBox))
          .first,
    );
    expect(
      painted.width,
      closeTo(MixarThemeData.dark().style.borderWidth, _kEpsilon),
    );
    expect(painted.height, closeTo(panel.height, _kEpsilon));
    // Both tiles are perfect circles, square to their cap.
    expect(cue.width, closeTo(cue.height, _kEpsilon));
    expect(playTile.width, closeTo(playTile.height, _kEpsilon));
    expect(cue.width, closeTo(playTile.width, _kEpsilon));
    expect(cue.width, 72, reason: 'the width cap sets the diameter here');
    expect(
      playTile.top - cue.bottom,
      closeTo(_kRowGap, _kEpsilon),
      reason: 'the pair keeps its fixed gap',
    );
    // Stacked as a pair, centred in the pads area.
    expect(
      cue.top - panel.top,
      closeTo(panel.bottom - playTile.bottom, _kEpsilon),
    );
    expect(playTile.top, greaterThan(cue.bottom - _kEpsilon));
    // Same column; the play glyph carries a 1px optical nudge.
    expect((cue.center.dx - play.center.dx).abs(), lessThan(1.5));
    expect(play.center.dy, greaterThan(cue.center.dy));
    // The pads keep the space the pair gives up.
    expect(panel.height, greaterThan(120));
    // The panel reaches the deck pane's bottom: no foot clearance.
    expect(panel.bottom, closeTo(290, _kEpsilon));

    // Tiled pads carry no bottom border, and the transport sits beside them:
    // the panel draws that closing hairline itself, on its own bounds.
    final ancestors = find.ancestor(
      of: find.byType(DeckPerformancePanel),
      matching: find.byType(DecoratedBox),
    );
    Border? foot;
    for (var i = 0; i < ancestors.evaluate().length; i++) {
      final decoration = tester
          .widget<DecoratedBox>(ancestors.at(i))
          .decoration;
      final border = decoration is BoxDecoration ? decoration.border : null;
      if (border is! Border || border.bottom == BorderSide.none) {
        continue;
      }
      final rect = tester.getRect(ancestors.at(i));
      if ((rect.width - panel.width).abs() < _kEpsilon &&
          (rect.height - panel.height).abs() < _kEpsilon) {
        foot = border;
        break;
      }
    }
    expect(foot, isNotNull);
    expect(foot!.bottom.color, MixarThemeData.dark().colors.border);
    expect(foot.top, BorderSide.none);
    expect(foot.left, BorderSide.none);
    expect(foot.right, BorderSide.none);
  });

  testWidgets('right-hand deck puts the compact transport left of the panel', (
    tester,
  ) async {
    await pumpDeck(tester, size: const Size(370, 290), deckId: 1);

    final panel = tester.getRect(find.byType(DeckPerformancePanel));
    final cue = tester.getRect(find.byType(DeckCueButton));
    final playTile = tester.getRect(
      find
          .ancestor(
            of: find.byIcon(LucideIcons.play),
            matching: find.byType(DecoratedBox),
          )
          .first,
    );
    final board = tester.getRect(find.byType(DeckTrackInfo));

    // Transport on the left, panel to its right.
    expect(cue.right, lessThanOrEqualTo(panel.left + _kEpsilon));
    expect(playTile.right, lessThanOrEqualTo(panel.left + _kEpsilon));
    // The pair keeps the deck's outer gutter and is centred in its column.
    expect(cue.left, board.left + _kSideInset);
    // The divider sits between the transport and the panel.
    final divider = tester.getRect(
      find.byKey(const ValueKey('deck-performance-transport-divider')),
    );
    expect(divider.center.dx, greaterThan(cue.right));
    expect(divider.center.dx, lessThan(panel.left));
    expect(divider.left, closeTo(cue.right + _kSideInset, _kEpsilon));
    // The two circles keep their size and stack in deck order.
    expect(cue.width, closeTo(playTile.width, _kEpsilon));
    expect(cue.center.dy, greaterThan(playTile.center.dy));
  });

  testWidgets('a narrow compact deck keeps the panel usable', (tester) async {
    await pumpDeck(tester, size: const Size(205, 300));

    expect(tester.takeException(), isNull);

    final panel = tester.getRect(find.byType(DeckPerformancePanel));
    final cue = tester.getRect(find.byType(DeckCueButton));

    // Too narrow for the side column: the transport takes the bottom row and
    // the panel keeps the deck's full width minus its margins.
    expect(cue.top, greaterThanOrEqualTo(panel.bottom - _kEpsilon));
    expect(panel.width, greaterThan(96));
  });

  testWidgets('a width-limited deck shrinks the circles to keep the pads', (
    tester,
  ) async {
    await pumpDeck(tester, size: const Size(294, 290));

    final rect = rects(tester);
    final panel = rect.panel;
    final cue = rect.cue;
    final playTile = tester.getRect(
      find
          .ancestor(
            of: find.byIcon(LucideIcons.play),
            matching: find.byType(DecoratedBox),
          )
          .first,
    );

    // The width budget, not the height cap, sets the circles here: the pads
    // keep their 120 px floor and their full height instead of a bottom row.
    expect(cue.left, greaterThanOrEqualTo(panel.right - _kEpsilon));
    expect(panel.width, greaterThanOrEqualTo(120 - _kEpsilon));
    expect(cue.width, closeTo(60, _kEpsilon));
    expect(cue.width, closeTo(cue.height, _kEpsilon));
    expect(cue.width, closeTo(playTile.width, _kEpsilon));
  });

  testWidgets('the stacked breakpoint budgets the whole transport block', (
    tester,
  ) async {
    // Mirrors the stacked footprint in deck_panel.dart: the panel floor, the
    // buttons, both row gaps, and the divider hairline.
    const panelFloor = 150.0;
    const buttonHeight = 36.0;
    final borderWidth = MixarThemeData.dark().style.borderWidth;
    final threshold = panelFloor + buttonHeight + _kRowGap * 2 + borderWidth;

    // Read the fixed deck chrome (track info) above the panel.
    await pumpDeck(tester, size: const Size(294, 560));
    final chrome = tester.getRect(find.byType(DeckTrackInfo)).height;

    // One pixel below the threshold: compact, so the panel keeps full height.
    await pumpDeck(tester, size: Size(294, threshold + chrome - 1));
    var cue = tester.getRect(find.byType(DeckCueButton));
    var panel = tester.getRect(find.byType(DeckPerformancePanel));
    expect(cue.width, closeTo(cue.height, _kEpsilon));
    expect(panel.height, greaterThan(190));

    // At the threshold: stacked, and the panel still keeps its floor.
    await pumpDeck(tester, size: Size(294, threshold + chrome));
    cue = tester.getRect(find.byType(DeckCueButton));
    panel = tester.getRect(find.byType(DeckPerformancePanel));
    expect(cue.width, greaterThan(cue.height * 1.5));
    expect(panel.height, greaterThanOrEqualTo(150 - _kEpsilon));
  });

  testWidgets('a loaded narrow deck keeps its time row inside', (tester) async {
    await pumpDeck(
      tester,
      size: const Size(205, 300),
      seed: const EngineUiSnapshot(
        running: true,
        trackPaths: {0: '/song.wav'},
        durationMs: {0: 205000},
      ),
    );

    expect(tester.takeException(), isNull);
    expect(
      find.text('-3:25'),
      findsOneWidget,
      reason: 'the narrow row keeps whole seconds and drops the total',
    );
    expect(find.text('3:25.0'), findsNothing);
    expect(
      tester.widget<Text>(find.text('-3:25')).style?.fontSize,
      MixarThemeData.dark().typography.body.xs.fontSize,
      reason: 'the compact readout steps down a size so it paints in full',
    );
  });

  testWidgets('tall deck keeps the full-width transport row under the panel', (
    tester,
  ) async {
    await pumpDeck(tester, size: const Size(370, 560));

    final (:panel, :cue, :play) = rects(tester);

    // Side by side under the panel, spanning the deck width.
    expect(cue.top, greaterThanOrEqualTo(panel.bottom - _kEpsilon));
    expect(play.top, greaterThanOrEqualTo(panel.bottom - _kEpsilon));
    expect(cue.left, lessThan(panel.right));
    expect(play.right, greaterThan(panel.center.dx));

    // The divider hugs the panel's bottom edge.
    final dividerFinder = find.byKey(
      const ValueKey('deck-performance-transport-divider'),
    );
    expect(dividerFinder, findsOneWidget);
    final divider = tester.getRect(dividerFinder);
    expect(divider.top, closeTo(panel.bottom, _kEpsilon));
    expect(divider.center.dy, lessThan(cue.top));
    final painted = tester.getRect(
      find
          .descendant(of: dividerFinder, matching: find.byType(ColoredBox))
          .first,
    );
    expect(
      painted.height,
      closeTo(MixarThemeData.dark().style.borderWidth, _kEpsilon),
    );
    // The divider spans the deck's full width, like the panel above it.
    expect(painted.width, closeTo(panel.width, _kEpsilon));
    expect(divider.left, closeTo(panel.left, _kEpsilon));
    expect(divider.right, closeTo(panel.right, _kEpsilon));
    // The buttons keep a side gutter inside the full-bleed deck body.
    final playSurface = find
        .ancestor(
          of: find.byIcon(LucideIcons.play),
          matching: find.byType(DecoratedBox),
        )
        .first;
    expect(cue.left, _kSideInset);
    final board = tester.getRect(find.byType(DeckTrackInfo));
    // The panel runs flush to the deck's content edges, with no side margin.
    expect(panel.left, board.left);
    expect(panel.right, board.right);
    expect(tester.getRect(playSurface).right, board.right - _kSideInset);
    // The row keeps equal space above and below the controls.
    final playRect = tester.getRect(playSurface);
    expect(playRect.top - divider.bottom, closeTo(_kRowGap, _kEpsilon));
    expect(560 - playRect.bottom, closeTo(_kRowGap, _kEpsilon));
    // Half-width each, not squares.
    expect(cue.width, greaterThan(cue.height * 1.5));
  });

  testWidgets('transport chrome is fully rounded in both shapes', (
    tester,
  ) async {
    BorderRadius chromeOf(Finder surface) =>
        (tester.widget<DecoratedBox>(surface).decoration as BoxDecoration)
                .borderRadius!
            as BorderRadius;

    final cueSurface = find
        .descendant(
          of: find.byType(DeckCueButton),
          matching: find.byType(DecoratedBox),
        )
        .first;
    final playSurface = find
        .ancestor(
          of: find.byIcon(LucideIcons.play),
          matching: find.byType(DecoratedBox),
        )
        .first;

    final surfaces = [(cueSurface, 'Cue'), (playSurface, 'Play')];

    // Short deck: the pair stacks beside the pads as square tiles.
    await pumpDeck(tester, size: const Size(370, 316));
    for (final (surface, what) in surfaces) {
      final radius = chromeOf(surface).topLeft.x;
      final size = tester.getSize(surface);
      expect(
        radius,
        greaterThanOrEqualTo(size.shortestSide / 2),
        reason: '$what keeps fully rounded ends while it is a tile',
      );
    }

    // Tall deck: the same pair spans the row as wide bars, still fully
    // rounded.
    await pumpDeck(tester, size: const Size(370, 560));
    for (final (surface, what) in surfaces) {
      expect(
        chromeOf(surface),
        MixarThemeData.dark().style.borderRadius.pill,
        reason: '$what stays fully rounded as a wide bar',
      );
      expect(
        tester.getSize(surface).width,
        greaterThan(tester.getSize(surface).height * 1.5),
        reason: 'the bar is wide, not a tile',
      );
    }
  });
}
