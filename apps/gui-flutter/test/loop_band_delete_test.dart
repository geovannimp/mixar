import 'package:flutter/gestures.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/providers/library/providers.dart';
import 'package:gui_flutter/providers/mixer/engine_providers.dart';
import 'package:gui_flutter/src/rust/api/event_mirrors.dart'
    show LoopRegion, SavedLoop;
import 'package:gui_flutter/widgets/mixer/waveform/overview_strip.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';
import 'package:material_ui/material_ui.dart';

import 'support/fake_engine_transport.dart';
import 'support/fake_library_transport.dart';
import 'support/mixar_material_app.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  const loop = SavedLoop(slot: 0, inMs: 2000, outMs: 4000);
  const stripWidth = 300.0;
  const stripHeight = 28.0;

  Future<({FakeEngineTransport engine, FakeLibraryTransport library})>
  pumpStrip(WidgetTester tester, {LoopRegion? activeLoop}) async {
    final engine = FakeEngineTransport();
    final library = FakeLibraryTransport();
    final theme = MixarThemeData.dark();
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          engineTransportProvider.overrideWith((ref) async => engine),
          libraryTransportProvider.overrideWith((ref) async => library),
          deckTrackIdProvider.overrideWith((ref, deckId) => 'track-1'),
          deckDurationMsProvider.overrideWith((ref, deckId) => 10000),
          deckPositionMsProvider.overrideWith((ref, deckId) => 0),
          deckSkeletonProvider.overrideWith((ref, deckId) => false),
          deckHotCuesProvider.overrideWith((ref, deckId) => const []),
          deckSavedLoopsProvider.overrideWith((ref, deckId) => const [loop]),
          deckActiveLoopProvider.overrideWith((ref, deckId) => activeLoop),
        ],
        child: mixarTestMaterialApp(
          theme: theme,
          home: const Scaffold(
            body: Center(
              child: SizedBox(
                width: stripWidth,
                height: stripHeight,
                child: OverviewStrip(deckId: 0),
              ),
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();
    return (engine: engine, library: library);
  }

  Offset centerOfLoop(WidgetTester tester) {
    final topLeft = tester.getTopLeft(find.byType(OverviewStrip));
    // Loop center is 3000 ms of a 10000 ms track.
    return topLeft + const Offset(stripWidth * 0.3, stripHeight / 2);
  }

  testWidgets('secondary tap on a saved band opens Delete loop', (
    tester,
  ) async {
    await pumpStrip(tester);

    await tester.tapAt(centerOfLoop(tester), buttons: kSecondaryButton);
    await tester.pumpAndSettle();

    expect(find.text('Delete loop'), findsOneWidget);
  });

  testWidgets('long-press on a saved band opens Delete loop', (tester) async {
    await pumpStrip(tester);

    await tester.longPressAt(centerOfLoop(tester));
    await tester.pumpAndSettle();

    expect(find.text('Delete loop'), findsOneWidget);
  });

  testWidgets('choosing Delete calls deleteLoop for the hit slot', (
    tester,
  ) async {
    final fake = await pumpStrip(tester);

    await tester.tapAt(centerOfLoop(tester), buttons: kSecondaryButton);
    await tester.pumpAndSettle();
    await tester.tap(find.text('Delete loop'));
    await tester.pumpAndSettle();

    expect(fake.library.deleteLoopCalls, [('track-1', loop.slot)]);
  });

  testWidgets('deleting the active loop exits it first', (tester) async {
    final fake = await pumpStrip(
      tester,
      activeLoop: const LoopRegion(inMs: 2000, outMs: 4000, active: true),
    );

    await tester.tapAt(centerOfLoop(tester), buttons: kSecondaryButton);
    await tester.pumpAndSettle();
    await tester.tap(find.text('Delete loop'));
    await tester.pumpAndSettle();

    expect(fake.engine.exitLoopCalls, [0]);
    expect(fake.library.deleteLoopCalls, [('track-1', loop.slot)]);
  });

  testWidgets('shift+hover turns the band into a delete affordance', (
    tester,
  ) async {
    await pumpStrip(tester);
    await tester.sendKeyDownEvent(LogicalKeyboardKey.shiftLeft);
    await tester.pump();

    final mouse = await tester.createGesture(kind: PointerDeviceKind.mouse);
    await mouse.addPointer(location: centerOfLoop(tester));
    await tester.sendEventToBinding(
      PointerHoverEvent(position: centerOfLoop(tester) + const Offset(1, 0)),
    );
    await tester.pump();

    expect(find.byIcon(LucideIcons.trash2), findsOneWidget);
    await tester.sendKeyUpEvent(LogicalKeyboardKey.shiftLeft);
  });

  testWidgets('shift+hover click deletes without the menu', (tester) async {
    final fake = await pumpStrip(tester);
    final position = centerOfLoop(tester);
    await tester.sendKeyDownEvent(LogicalKeyboardKey.shiftLeft);
    await tester.pump();

    final mouse = await tester.createGesture(kind: PointerDeviceKind.mouse);
    await mouse.addPointer(location: position);
    await tester.sendEventToBinding(
      PointerHoverEvent(position: position + const Offset(1, 0)),
    );
    await tester.pump();
    await tester.tapAt(position);
    await tester.pumpAndSettle();

    expect(fake.library.deleteLoopCalls, [('track-1', loop.slot)]);
    expect(find.text('Delete loop'), findsNothing);
    await tester.sendKeyUpEvent(LogicalKeyboardKey.shiftLeft);
  });
}
