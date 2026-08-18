import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:forui/forui.dart';
import 'package:gui_flutter/library/providers.dart';
import 'package:gui_flutter/library/track_table_pane.dart';
import 'package:gui_flutter/mixer/engine_providers.dart';
import 'package:gui_flutter/shell/desktop.dart';
import 'package:gui_flutter/shell/material_theme.dart';
import 'package:gui_flutter/settings/settings_defaults.dart';
import 'package:gui_flutter/settings/settings_providers.dart';
import 'package:gui_flutter/src/rust/api/library.dart';
import 'package:material_ui/material_ui.dart';
import 'package:super_drag_and_drop/super_drag_and_drop.dart';

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
    album: null,
    genre: null,
    bpm: 128,
    key: '8A',
    durationMs: 180000,
    path: '/tmp/samples/demo.wav',
  );

  testWidgets(
    'row drag attaches after engine starts without changing collection',
    (tester) async {
      debugOverrideDesktopWindow = false;
      addTearDown(() => debugOverrideDesktopWindow = null);

      final theme = FTheme.neutral.light.desktop;
      tester.view.physicalSize = const Size(800, 600);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);

      await tester.pumpWidget(
        ProviderScope(
          overrides: [
            collectionsProvider.overrideWith((ref) async => [collection]),
            collectionTracksProvider.overrideWith((ref) async => [track]),
            libraryEventsBootstrapProvider.overrideWith((ref) {}),
            appSettingsProvider.overrideWith((ref) async => defaultAppSettings()),
          ],
          child: MaterialApp(
            theme: materialUiThemeFromForui(theme),
            builder: (context, child) => MaterialUiCompatibilityBridge(
              // ignore: deprecated_member_use
              child: FTheme(data: theme, child: child!),
            ),
            home: const Scaffold(
              body: SizedBox(
                width: 800,
                height: 600,
                child: TrackTablePane(),
              ),
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();

      expect(find.text('Demo Track'), findsOneWidget);
      expect(find.byType(DragItemWidget), findsNothing);

      final container = ProviderScope.containerOf(
        tester.element(find.byType(TrackTablePane)),
      );
      container.read(engineUiProvider.notifier).setRunning(true);
      await tester.pumpAndSettle();

      expect(find.byType(DragItemWidget), findsWidgets);
    },
  );
}
