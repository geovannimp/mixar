import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/utils/library_list_layout.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/core/utils/track_drag.dart';
import 'package:gui_flutter/providers/mixer/engine_providers.dart';
import 'package:gui_flutter/providers/settings/settings_providers.dart';
import 'package:gui_flutter/widgets/mixer/deck_track_info.dart';
import 'package:gui_flutter/widgets/mixer/track_drop_zone.dart';
import 'package:gui_flutter/widgets/shell/m_card.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:material_ui/material_ui.dart' show Scaffold;

import 'helpers/recording_deck_load_transport.dart';
import 'support/fake_engine_transport.dart';
import 'support/mixar_material_app.dart';
import 'support/row_drag.dart';

const _sourceKey = Key('drag-source');
const _cardKey = Key('deck-info');

class _Running extends Notifier<bool> {
  @override
  bool build() => true;

  void set(bool value) => state = value;
}

final _runningProvider = NotifierProvider<_Running, bool>(_Running.new);

void main() {
  Future<(RecordingDeckLoadTransport, ProviderContainer)> pumpCard(
    WidgetTester tester, {
    required MixarThemeData theme,
    required FakeEngineTransport engine,
  }) async {
    final loads = RecordingDeckLoadTransport();
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          engineTransportProvider.overrideWith((ref) async => engine),
          engineRunningProvider.overrideWith(
            (ref) => ref.watch(_runningProvider),
          ),
          deckLoadTransportProvider.overrideWith((ref) async => loads),
          appSettingsProvider.overrideWith((ref) async => defaultAppSettings()),
          audioDevicesProvider.overrideWith((ref) async => const []),
          samplerBanksProvider.overrideWith((ref) async => const []),
        ],
        child: mixarTestMaterialApp(
          theme: theme,
          home: Scaffold(
            body: Stack(
              children: [
                Positioned(
                  top: 20,
                  left: 20,
                  child: libraryTrackDragSource(
                    payload: const TrackDragPayload(
                      source: TrackDragSource.library,
                      trackId: 't1',
                      path: '/lib/a.wav',
                      title: 'Alpha',
                    ),
                    child: const ColoredBox(
                      key: _sourceKey,
                      color: Color(0xFF000000),
                      child: SizedBox(width: 40, height: 40),
                    ),
                  ),
                ),
                const Positioned(
                  top: 200,
                  left: 20,
                  child: TrackDropZone(
                    deckId: 0,
                    child: SizedBox(
                      key: _cardKey,
                      width: 360,
                      height: 130,
                      child: DeckTrackInfo(
                        deckId: 0,
                        hasTrack: false,
                        title: null,
                      ),
                    ),
                  ),
                ),
              ],
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();
    final container = ProviderScope.containerOf(
      tester.element(find.byType(DeckTrackInfo)),
    );
    return (loads, container);
  }

  Border cardBorder(WidgetTester tester) {
    final box = tester.widget<DecoratedBox>(
      find
          .descendant(
            of: find.byType(MCard),
            matching: find.byType(DecoratedBox),
          )
          .first,
    );
    return (box.decoration as BoxDecoration).border! as Border;
  }

  Finder outlineFinder(MixarThemeData theme) => find.descendant(
    of: find.byType(MCard),
    matching: find.byWidgetPredicate(
      (widget) =>
          widget is DecoratedBox &&
          widget.decoration is BoxDecoration &&
          (widget.decoration as BoxDecoration).border is Border &&
          ((widget.decoration as BoxDecoration).border! as Border).top.color ==
              theme.colors.primary,
    ),
  );

  DecoratedBox outline(WidgetTester tester, MixarThemeData theme) =>
      tester.widget<DecoratedBox>(outlineFinder(theme));

  Border outlineBorder(WidgetTester tester, MixarThemeData theme) =>
      (outline(tester, theme).decoration as BoxDecoration).border! as Border;

  testWidgets(
    'the track info card closes its border while a track hangs over',
    (tester) async {
      final theme = MixarThemeData.dark();
      final (loads, _) = await pumpCard(
        tester,
        theme: theme,
        engine: FakeEngineTransport(),
      );

      expect(cardBorder(tester).top.style, BorderStyle.none);
      expect(cardBorder(tester).bottom.color, theme.colors.border);
      expect(outlineFinder(theme), findsNothing);

      final gesture = await startRowDrag(
        tester,
        tester.getCenter(find.byKey(_sourceKey)),
      );
      await gesture.moveTo(tester.getCenter(find.byKey(_cardKey)));
      await tester.pump();
      for (final side in [
        outlineBorder(tester, theme).top,
        outlineBorder(tester, theme).right,
        outlineBorder(tester, theme).bottom,
        outlineBorder(tester, theme).left,
      ]) {
        expect(side.color, theme.colors.primary);
        expect(side.width, theme.style.borderWidth);
      }
      // The card stays full-bleed, but the outline moves one hairline in on
      // each side: the window frame owns the card's outermost column on the
      // outer decks and would otherwise crop the closed outline there.
      final cardRect = tester.getRect(find.byKey(_cardKey));
      final outlineRect = tester.getRect(outlineFinder(theme));
      expect(outlineRect.left, cardRect.left + theme.style.borderWidth);
      expect(outlineRect.right, cardRect.right - theme.style.borderWidth);
      expect(outlineRect.top, cardRect.top);
      expect(outlineRect.bottom, cardRect.bottom);

      // The closed outline makes the card reserve its side hairlines; the
      // title-row math must account for them or the cluster grows into the
      // title and pushes it below its floor.
      expect(
        tester.getRect(find.byKey(const ValueKey('deck-title-artist'))).width,
        56,
        reason: 'the closed outline must not steal room from the title',
      );

      await gesture.moveTo(const Offset(380, 20));
      await tester.pump();
      expect(outlineFinder(theme), findsNothing);

      await gesture.moveTo(tester.getCenter(find.byKey(_cardKey)));
      await tester.pump();
      await gesture.up();
      await tester.pumpAndSettle();

      expect(loads.libraryLoads, [(0, 't1')]);
    },
  );

  testWidgets('a stopped engine clears a stale drop hover', (tester) async {
    final theme = MixarThemeData.dark();
    final (loads, container) = await pumpCard(
      tester,
      theme: theme,
      engine: FakeEngineTransport(),
    );

    final gesture = await startRowDrag(
      tester,
      tester.getCenter(find.byKey(_sourceKey)),
    );
    await gesture.moveTo(tester.getCenter(find.byKey(_cardKey)));
    await tester.pump();
    expect(outlineFinder(theme), findsOneWidget);

    container.read(_runningProvider.notifier).set(false);
    await tester.pumpAndSettle();
    expect(
      outlineFinder(theme),
      findsNothing,
      reason: 'the hover outline must not outlive the drop target',
    );

    container.read(_runningProvider.notifier).set(true);
    await tester.pumpAndSettle();
    expect(
      outlineFinder(theme),
      findsNothing,
      reason: 'restarting the engine does not restore a hover nobody made',
    );

    await gesture.up();
    await tester.pumpAndSettle();
    expect(loads.libraryLoads, isEmpty);
  });
}
