import 'package:flutter/gestures.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/library/providers.dart';
import 'package:gui_flutter/library/track_list.dart';
import 'package:gui_flutter/library/track_list_pane.dart';
import 'package:gui_flutter/mixer/engine_providers.dart';
import 'package:gui_flutter/mixer/engine_ui.dart';
import 'package:gui_flutter/mixer/track_drag.dart';
import 'package:gui_flutter/shell/app_button.dart';
import 'package:gui_flutter/shell/material_theme.dart';
import 'package:gui_flutter/shell/mixar_theme.dart';
import 'package:gui_flutter/src/rust/api/library.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';
import 'package:material_ui/material_ui.dart';

import 'support/mixar_material_app.dart';

class _RunningEngineUi extends EngineUi {
  @override
  EngineUiSnapshot build() =>
      const EngineUiSnapshot(running: true, trackPaths: {});
}

AppButton _loadChip(WidgetTester tester, String letter) {
  return tester.widget<AppButton>(
    find.byWidgetPredicate(
      (widget) =>
          widget is AppButton && widget.semanticsLabel == 'Load to $letter',
    ),
  );
}

void main() {
  const track = LibraryTrackSummary(
    id: 't1',
    displayName: 'Track',
    title: 'Track',
    path: '/tmp/t1.wav',
  );
  const payload = TrackDragPayload(
    source: TrackDragSource.library,
    trackId: 't1',
    path: '/tmp/t1.wav',
    title: 'Track',
  );

  Future<ProviderContainer> pumpMenu(
    WidgetTester tester, {
    bool running = false,
    bool stemsGenerating = false,
    double width = 200,
  }) async {
    final theme = MixarThemeData.dark();
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          driveResolvedByPathProvider.overrideWith(
            (ref) async => const <String, LibraryTrackSummary>{},
          ),
          if (running) engineUiProvider.overrideWith(_RunningEngineUi.new),
        ],
        child: MaterialApp(
          theme: materialUiThemeFromMixar(theme),
          builder: mixarMaterialAppBuilder(theme),
          home: Scaffold(
            body: SizedBox(
              width: width,
              height: 36,
              child: TrackListMenuButton(
                menuBuilder: (context, ref, dismiss) => buildTrackListMenuBody(
                  context: context,
                  ref: ref,
                  payload: payload,
                  // The real track items, so the menu under test is the one the
                  // library list renders.
                  extras: trackRowExtraMenuItems(context, ref, track, dismiss),
                  dismiss: dismiss,
                ),
              ),
            ),
          ),
        ),
      ),
    );
    final container = ProviderScope.containerOf(
      tester.element(find.byType(TrackListMenuButton)),
    );
    if (stemsGenerating) {
      container
          .read(stemGeneratingTrackIdsProvider.notifier)
          .setGenerating(track.id, true);
      await tester.pump();
    }
    return container;
  }

  testWidgets('⋯ icon fits a 40px table cell', (tester) async {
    Object? overflow;
    final previous = FlutterError.onError;
    FlutterError.onError = (details) {
      if (details.toString().contains('overflowed')) {
        overflow = details.exception;
      }
      previous?.call(details);
    };
    addTearDown(() => FlutterError.onError = previous);

    await pumpMenu(tester, width: 40);
    expect(overflow, isNull);
    expect(find.byIcon(LucideIcons.ellipsisVertical), findsOneWidget);
  });

  testWidgets('Analyze is enabled for library tracks', (tester) async {
    await pumpMenu(tester);
    await tester.tap(find.byIcon(LucideIcons.ellipsisVertical));
    await tester.pumpAndSettle();
    expect(find.text('Analyze'), findsOneWidget);
    expect(tester.getSize(find.text('Load to deck')).width, lessThan(300));
  });

  testWidgets('Generate stems is its own action for library tracks', (
    tester,
  ) async {
    await pumpMenu(tester);
    await tester.tap(find.byIcon(LucideIcons.ellipsisVertical));
    await tester.pumpAndSettle();
    // Stems no longer ride along with Analyze, so they get a separate item.
    expect(find.text('Generate stems'), findsOneWidget);
  });

  testWidgets('Generate stems reports progress and blocks a second run', (
    tester,
  ) async {
    await pumpMenu(tester, stemsGenerating: true);
    await tester.tap(find.byIcon(LucideIcons.ellipsisVertical));
    await tester.pumpAndSettle();
    expect(find.text('Generating stems…'), findsOneWidget);
    expect(find.text('Generate stems'), findsNothing);
  });

  testWidgets('Load to A/B is disabled when the engine is stopped', (
    tester,
  ) async {
    await pumpMenu(tester);
    await tester.tap(find.byIcon(LucideIcons.ellipsisVertical));
    await tester.pumpAndSettle();
    expect(find.text('Load to deck'), findsOneWidget);
    expect(_loadChip(tester, 'A').onPress, isNull);
    expect(_loadChip(tester, 'B').onPress, isNull);
  });

  testWidgets('right-click opens the track actions menu', (tester) async {
    await pumpMenu(tester);
    await tester.tap(
      find.byIcon(LucideIcons.ellipsisVertical),
      buttons: kSecondaryButton,
    );
    await tester.pumpAndSettle();
    expect(find.text('Load to deck'), findsOneWidget);
  });

  testWidgets('Load to A/B is enabled when the engine is running', (
    tester,
  ) async {
    await pumpMenu(tester, running: true);
    await tester.tap(find.byIcon(LucideIcons.ellipsisVertical));
    await tester.pumpAndSettle();
    expect(find.text('Load to deck'), findsOneWidget);
    expect(_loadChip(tester, 'A').onPress, isNotNull);
    expect(_loadChip(tester, 'B').onPress, isNotNull);
  });

  test('collection tracks stay in-library when id equals path', () {
    const track = LibraryTrackSummary(
      id: '/music/a.wav',
      displayName: 'a.wav',
      path: '/music/a.wav',
    );
    expect(trackIsInLibrary(track, tab: LibrarySourceTab.collections), isTrue);
    expect(trackIsInLibrary(track, tab: LibrarySourceTab.drive), isFalse);
    expect(
      trackIsInLibrary(
        track,
        tab: LibrarySourceTab.drive,
        driveResolvedByPath: {track.path: track},
      ),
      isTrue,
    );
  });
}
