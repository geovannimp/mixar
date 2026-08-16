import 'package:gui_flutter/shell/material_theme.dart';
import 'package:material_ui/material_ui.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:forui/forui.dart';
import 'package:gui_flutter/library/providers.dart';
import 'package:gui_flutter/library/track_table_pane.dart';
import 'package:gui_flutter/src/rust/api/library.dart';

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

  test('collection tracks stay in-library when id equals path', () {
    final track = LibraryTrackSummary(
      id: '/music/a.wav',
      displayName: 'a.wav',
      path: '/music/a.wav',
    );
    expect(
      trackIsInLibrary(track, tab: LibrarySourceTab.collections),
      isTrue,
    );
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
