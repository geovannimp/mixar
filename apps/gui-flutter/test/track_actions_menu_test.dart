import 'package:gui_flutter/shell/material_theme.dart';
import 'package:material_ui/material_ui.dart';
import 'package:flutter/gestures.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:forui/forui.dart';
import 'package:gui_flutter/library/providers.dart';
import 'package:gui_flutter/library/track_table_pane.dart';
import 'package:gui_flutter/mixer/engine_providers.dart';
import 'package:gui_flutter/mixer/engine_ui.dart';
import 'package:gui_flutter/src/rust/api/library.dart';

class _RunningEngineUi extends EngineUi {
  @override
  EngineUiSnapshot build() => const EngineUiSnapshot(running: true, titles: {});
}

FButton _loadChip(WidgetTester tester, String letter) {
  return tester.widget<FButton>(
    find.byWidgetPredicate(
      (widget) =>
          widget is FButton && widget.semanticsLabel == 'Load to $letter',
    ),
  );
}

void main() {
  Future<void> pumpMenu(
    WidgetTester tester, {
    required bool inLibrary,
    double width = 200,
  }) async {
    final theme = FTheme.neutral.dark.desktop;
    await tester.pumpWidget(
      ProviderScope(
        child: MaterialApp(
          theme: materialUiThemeFromForui(theme),
          builder: (context, child) => MaterialUiCompatibilityBridge(
            // ignore: deprecated_member_use
            child: FTheme(data: theme, child: child!),
          ),
          home: Scaffold(
            body: SizedBox(
              width: width,
              height: 36,
              child: TrackActionsMenu(
                trackId: 't1',
                path: '/tmp/t1.wav',
                title: 'Track',
                inLibrary: inLibrary,
                analyzing: false,
              ),
            ),
          ),
        ),
      ),
    );
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

    await pumpMenu(tester, inLibrary: true, width: 40);
    expect(overflow, isNull);
    expect(find.byIcon(FLucideIcons.ellipsisVertical), findsOneWidget);
  });

  testWidgets('Analyze is enabled for library tracks', (tester) async {
    await pumpMenu(tester, inLibrary: true);
    await tester.tap(find.byIcon(FLucideIcons.ellipsisVertical));
    await tester.pumpAndSettle();
    final item = tester.widget<FItem>(find.widgetWithText(FItem, 'Analyze'));
    expect(item.enabled, isNot(false));
    expect(item.onPress, isNotNull);
  });

  testWidgets('Load to A/B is disabled when the engine is stopped', (
    tester,
  ) async {
    await pumpMenu(tester, inLibrary: true);
    await tester.tap(find.byIcon(FLucideIcons.ellipsisVertical));
    await tester.pumpAndSettle();
    expect(find.text('Load to deck'), findsOneWidget);
    expect(_loadChip(tester, 'A').onPress, isNull);
    expect(_loadChip(tester, 'B').onPress, isNull);
  });

  testWidgets('right-click opens the track actions menu', (tester) async {
    await pumpMenu(tester, inLibrary: true);
    await tester.tap(
      find.byIcon(FLucideIcons.ellipsisVertical),
      buttons: kSecondaryButton,
    );
    await tester.pumpAndSettle();
    expect(find.text('Load to deck'), findsOneWidget);
  });

  testWidgets('Load to A/B is enabled when the engine is running', (
    tester,
  ) async {
    final theme = FTheme.neutral.dark.desktop;
    await tester.pumpWidget(
      ProviderScope(
        overrides: [engineUiProvider.overrideWith(_RunningEngineUi.new)],
        child: MaterialApp(
          theme: materialUiThemeFromForui(theme),
          builder: (context, child) => MaterialUiCompatibilityBridge(
            // ignore: deprecated_member_use
            child: FTheme(data: theme, child: child!),
          ),
          home: const Scaffold(
            body: SizedBox(
              width: 200,
              height: 36,
              child: TrackActionsMenu(
                trackId: 't1',
                path: '/tmp/t1.wav',
                title: 'Track',
                inLibrary: true,
                analyzing: false,
              ),
            ),
          ),
        ),
      ),
    );
    await tester.tap(find.byIcon(FLucideIcons.ellipsisVertical));
    await tester.pumpAndSettle();
    expect(find.text('Load to deck'), findsOneWidget);
    expect(_loadChip(tester, 'A').onPress, isNotNull);
    expect(_loadChip(tester, 'B').onPress, isNotNull);
  });

  test('collection tracks stay in-library when id equals path', () {
    final track = LibraryTrackSummary(
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
