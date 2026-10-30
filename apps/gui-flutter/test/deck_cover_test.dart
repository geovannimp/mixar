import 'package:flutter/gestures.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/models/engine_ui.dart';

import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/providers/library/drawer_providers.dart';
import 'package:gui_flutter/providers/mixer/engine_providers.dart';
import 'package:gui_flutter/providers/settings/settings_providers.dart';
import 'package:gui_flutter/widgets/mixer/deck_cover.dart';
import 'package:gui_flutter/widgets/mixer/deck_drop_hover.dart';
import 'package:gui_flutter/widgets/shell/app_tooltip.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';

import 'support/bus_events.dart';

import 'package:lucide_icons_flutter/lucide_icons.dart';

import 'support/fake_engine_transport.dart';
import 'support/mixar_material_app.dart';

const _deckId = 0;

/// A deck whose engine load is still running.
class _InFlightLoad extends DeckLoadInFlight {
  @override
  Map<int, int> build() => const {_deckId: 1};
}

/// Engine snapshot with `_deckId` loaded — the cover's loaded state.
class _LoadedEngineUi extends EngineUi {
  @override
  EngineUiSnapshot build() => applyEngineEvent(
    EngineUiSnapshot.empty,
    deckUpdatedEvent(
      deck: _deckId,
      update: deckUpdated(
        deck: _deckId,
        trackPath: '/music/a.wav',
        trackId: 't1',
        durationMs: 180000,
      ),
    ),
  );
}

/// Cover wired to real state, so unload flips it like in the app.
class _CoverHost extends ConsumerWidget {
  const new({this.dropHover = false});

  final bool dropHover;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final cover = SizedBox.square(
      dimension: 64,
      child: DeckCover(
        deckId: _deckId,
        hasTrack: ref.watch(deckHasTrackProvider(_deckId)),
      ),
    );
    return DeckDropHover(active: dropHover, child: cover);
  }
}

Widget _app({required Widget child}) => mixarTestMaterialApp(
  theme: MixarThemeData.dark(),
  home: Center(child: child),
);

/// Pumps one cover with the seams the feature needs.
Future<FakeEngineTransport> _pump(
  WidgetTester tester, {
  bool engineRunning = true,
  bool loaded = false,
  bool playing = false,
  bool dropHover = false,
  bool drawerMode = false,
  bool loadInFlight = false,
  FakeEngineTransport? engine,
}) async {
  final fakeEngine = engine ?? FakeEngineTransport();
  await tester.pumpWidget(
    ProviderScope(
      overrides: [
        engineTransportProvider.overrideWith((ref) async => fakeEngine),
        engineRunningProvider.overrideWithValue(engineRunning),
        appSettingsProvider.overrideWith(
          (ref) async => copyAppSettings(
            defaultAppSettings(),
            libraryDrawerEnabled: drawerMode,
          ),
        ),
        if (loaded) engineUiProvider.overrideWith(_LoadedEngineUi.new),
        if (loadInFlight)
          deckLoadInFlightProvider.overrideWith(_InFlightLoad.new),
        if (playing) deckPlayingProvider.overrideWith((ref, deckId) => true),
      ],
      child: _app(child: _CoverHost(dropHover: dropHover)),
    ),
  );
  await tester.pumpAndSettle();
  return fakeEngine;
}

/// The cover's provider container, for reading the library providers.
ProviderContainer _container(WidgetTester tester) =>
    ProviderScope.containerOf(tester.element(find.byType(DeckCover)));

/// The eject chip's animated box — its size is what hover grows.
Finder get _chipBox => find.descendant(
  of: find.byType(DeckCover),
  matching: find.byType(AnimatedContainer),
);

Future<TestGesture> _hoverCover(WidgetTester tester) async {
  final mouse = await tester.createGesture(kind: PointerDeviceKind.mouse);
  await mouse.addPointer();
  addTearDown(mouse.removePointer);
  await mouse.moveTo(tester.getCenter(find.byType(DeckCover)));
  await tester.pumpAndSettle();
  return mouse;
}

void main() {
  testWidgets('the empty cover opens the library drawer', (tester) async {
    await _pump(tester, drawerMode: true);

    expect(find.byIcon(LucideIcons.plus), findsOneWidget);
    expect(find.byIcon(LucideIcons.eject), findsNothing);

    await tester.tap(find.byType(DeckCover));
    await tester.pumpAndSettle();

    expect(_container(tester).read(libraryDrawerOpenProvider), isTrue);
    expect(find.byIcon(LucideIcons.plus), findsOneWidget);
  });

  testWidgets('with the docked library, the cover asks it into view', (
    tester,
  ) async {
    await _pump(tester);

    await tester.tap(find.byType(DeckCover));
    await tester.pumpAndSettle();

    final container = _container(tester);
    expect(container.read(libraryRevealProvider), 1);
    expect(container.read(libraryDrawerOpenProvider), isFalse);
  });

  testWidgets('hovering a loaded cover grows the eject chip to the cover', (
    tester,
  ) async {
    await _pump(tester, loaded: true);

    expect(tester.getSize(_chipBox), const Size(20, 20));
    expect(find.byIcon(LucideIcons.libraryBig), findsNothing);

    await _hoverCover(tester);

    expect(tester.getSize(_chipBox), const Size(64, 64));
    expect(find.byIcon(LucideIcons.libraryBig), findsNothing);
  });

  testWidgets('clicking a grown cover unloads the deck', (tester) async {
    final engine = await _pump(tester, loaded: true);

    await _hoverCover(tester);
    await tester.tap(find.byType(DeckCover));
    await tester.pumpAndSettle();

    expect(engine.unloads, [_deckId]);
    expect(find.byIcon(LucideIcons.eject), findsNothing);
    expect(find.byIcon(LucideIcons.plus), findsOneWidget);
  });

  testWidgets('a playing deck asks before it is unloaded', (tester) async {
    final engine = await _pump(tester, loaded: true, playing: true);

    await tester.tap(find.byType(DeckCover));
    await tester.pumpAndSettle();

    expect(find.text('Unload Deck A?'), findsOneWidget);
    expect(engine.unloads, isEmpty);

    await tester.tap(find.text('Unload'));
    await tester.pumpAndSettle();

    expect(engine.unloads, [_deckId]);
    expect(find.byIcon(LucideIcons.plus), findsOneWidget);
  });

  testWidgets('cancelling the confirm keeps a playing deck loaded', (
    tester,
  ) async {
    final engine = await _pump(tester, loaded: true, playing: true);

    await tester.tap(find.byType(DeckCover));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Cancel'));
    await tester.pumpAndSettle();

    expect(engine.unloads, isEmpty);
    expect(find.byIcon(LucideIcons.eject), findsOneWidget);
    expect(find.text('Unload Deck A?'), findsNothing);
  });

  testWidgets('a load started during the confirm still blocks the unload', (
    tester,
  ) async {
    final engine = await _pump(tester, loaded: true, playing: true);

    await tester.tap(find.byType(DeckCover));
    await tester.pumpAndSettle();
    expect(find.text('Unload Deck A?'), findsOneWidget);

    // A controller can start a load while the dialog is up.
    _container(tester)
        .read(deckLoadInFlightProvider.notifier)
        .set(_deckId, true);
    await tester.pumpAndSettle();

    await tester.tap(find.text('Unload'));
    await tester.pumpAndSettle();

    expect(engine.unloads, isEmpty);
    expect(find.text('Deck is busy — unload cancelled'), findsOneWidget);
  });

  testWidgets('a drag over the deck paints the drop hint on the artwork', (
    tester,
  ) async {
    await _pump(tester, loaded: true, dropHover: true);

    expect(find.byIcon(LucideIcons.arrowDownToLine), findsOneWidget);
  });

  testWidgets('an empty cover shows the drop hint too', (tester) async {
    await _pump(tester, dropHover: true);

    expect(find.byIcon(LucideIcons.arrowDownToLine), findsOneWidget);
    expect(find.byIcon(LucideIcons.plus), findsOneWidget);
  });

  testWidgets('a load in flight blocks unload until it lands', (tester) async {
    final engine = await _pump(tester, loaded: true, loadInFlight: true);

    await tester.tap(find.byType(DeckCover));
    await tester.pumpAndSettle();

    expect(engine.unloads, isEmpty);
    expect(find.text('Unload Deck A?'), findsNothing);
  });

  testWidgets('a stopped engine leaves the loaded cover inert', (tester) async {
    final engine = await _pump(tester, loaded: true, engineRunning: false);

    await tester.tap(find.byType(DeckCover));
    await tester.pumpAndSettle();

    expect(engine.unloads, isEmpty);
    expect(find.text('Unload Deck A?'), findsNothing);
    expect(find.byIcon(LucideIcons.eject), findsOneWidget);
  });

  testWidgets('a stopped engine still lets an empty cover open the library', (
    tester,
  ) async {
    await _pump(tester, engineRunning: false, drawerMode: true);

    // The deck installs no drop target while stopped, so the tip must not
    // invite a drag that would go nowhere.
    final tip = tester.widget<AppTooltip>(
      find.descendant(
        of: find.byType(DeckCover),
        matching: find.byType(AppTooltip),
      ),
    );
    expect(tip.description, 'Opens the library.');

    await tester.tap(find.byType(DeckCover));
    await tester.pumpAndSettle();

    expect(_container(tester).read(libraryDrawerOpenProvider), isTrue);
  });

  testWidgets('a running engine invites the drag in the cover tip', (
    tester,
  ) async {
    await _pump(tester);

    final tip = tester.widget<AppTooltip>(
      find.descendant(
        of: find.byType(DeckCover),
        matching: find.byType(AppTooltip),
      ),
    );
    expect(
      tip.description,
      'Opens the library. Drag a track here, or drop one.',
    );
  });
}
