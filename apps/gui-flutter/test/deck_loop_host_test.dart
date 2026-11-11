import 'dart:async';

import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/providers/library/providers.dart';
import 'package:gui_flutter/providers/mixer/engine_providers.dart';
import 'package:gui_flutter/src/rust/api/event_mirrors.dart'
    show LoopRegion, SavedLoop;
import 'package:gui_flutter/src/rust/api/library.dart' show LibraryTransport;
import 'package:gui_flutter/widgets/mixer/deck_loop_host.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:material_ui/material_ui.dart';

import 'support/fake_engine_transport.dart';
import 'support/fake_library_transport.dart';
import 'support/mixar_material_app.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  Future<void> pumpHost(
    WidgetTester tester, {
    required FakeEngineTransport engine,
    required Future<LibraryTransport> library,
    List<SavedLoop> savedLoops = const [],
    LoopRegion? activeLoop,
    int positionMs = 2000,
    String? trackId = 'track-1',
  }) async {
    final theme = MixarThemeData.dark();
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          engineTransportProvider.overrideWith((ref) async => engine),
          libraryTransportProvider.overrideWith((ref) => library),
          deckTrackIdProvider.overrideWith((ref, deckId) => trackId),
          deckPositionMsProvider.overrideWith((ref, deckId) => positionMs),
          deckSavedLoopsProvider.overrideWith((ref, deckId) => savedLoops),
          deckActiveLoopProvider.overrideWith((ref, deckId) => activeLoop),
          deckPendingLoopInMsProvider.overrideWith((ref, deckId) => null),
          deckBpmProvider.overrideWith((ref, deckId) => 120.0),
        ],
        child: mixarTestMaterialApp(
          theme: theme,
          home: const Scaffold(
            body: SizedBox(
              width: 320,
              child: DeckLoopHost(deckId: 0, hasTrack: true),
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();
  }

  testWidgets('shift+Loop deletes the saved loop under the playhead', (
    tester,
  ) async {
    final engine = FakeEngineTransport();
    final library = FakeLibraryTransport();
    await pumpHost(
      tester,
      engine: engine,
      library: Future.value(library),
      savedLoops: const [SavedLoop(slot: 1, inMs: 1000, outMs: 3000)],
      activeLoop: const LoopRegion(inMs: 1000, outMs: 3000, active: true),
    );

    await tester.sendKeyDownEvent(LogicalKeyboardKey.shiftLeft);
    await tester.pump();
    await tester.tap(find.text('Loop'));
    await tester.pumpAndSettle();
    await tester.sendKeyUpEvent(LogicalKeyboardKey.shiftLeft);

    expect(engine.exitLoopCalls, [0]);
    expect(library.deleteLoopCalls, [('track-1', 1)]);
  });

  testWidgets('Loop recalls a saved loop without shift', (tester) async {
    final engine = FakeEngineTransport();
    final library = FakeLibraryTransport();
    await pumpHost(
      tester,
      engine: engine,
      library: Future.value(library),
      savedLoops: const [SavedLoop(slot: 2, inMs: 1000, outMs: 3000)],
    );

    await tester.tap(find.text('Loop'));
    await tester.pumpAndSettle();

    expect(engine.savedLoopRecallCalls, [(0, 1000, 3000)]);
    expect(library.deleteLoopCalls, isEmpty);
  });

  testWidgets('Loop with no saved loop starts an auto loop', (tester) async {
    final engine = FakeEngineTransport();
    final library = FakeLibraryTransport();
    await pumpHost(
      tester,
      engine: engine,
      library: Future.value(library),
      positionMs: 500,
    );

    await tester.tap(find.text('Loop'));
    await tester.pumpAndSettle();

    expect(engine.autoLoopCalls, [(0, 4.0)]);
    expect(library.deleteLoopCalls, isEmpty);
  });

  testWidgets('hydrates a track that arrives before the library', (
    tester,
  ) async {
    final engine = FakeEngineTransport();
    final library = FakeLibraryTransport();
    final libraryReady = Completer<LibraryTransport>();
    await pumpHost(tester, engine: engine, library: libraryReady.future);

    expect(library.refreshTrackCalls, isEmpty);

    libraryReady.complete(library);
    await tester.pumpAndSettle();

    expect(library.refreshTrackCalls, ['track-1']);
  });
}
