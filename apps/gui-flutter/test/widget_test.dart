import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:forui/forui.dart';
import 'package:gui_flutter/library/providers.dart';
import 'package:gui_flutter/shell/app_shell.dart';
import 'package:gui_flutter/shell/desktop.dart';
import 'package:gui_flutter/src/rust/api/library.dart';

void main() {
  testWidgets('mixer shell shows core regions', (tester) async {
    debugOverrideDesktopWindow = false;
    addTearDown(() => debugOverrideDesktopWindow = null);

    final theme = FTheme.neutral.light.desktop;
    tester.view.physicalSize = const Size(1400, 900);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);

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

    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          collectionsProvider.overrideWith((ref) async => [collection]),
          collectionTracksProvider.overrideWith((ref) async {
            final id = ref.watch(activeCollectionIdProvider);
            if (id == null) {
              return const [];
            }
            return [track];
          }),
        ],
        child: MaterialApp(
          theme: theme.toApproximateMaterialTheme(),
          builder: (context, child) => FTheme(data: theme, child: child!),
          home: const AppShell(appTitle: 'Rust DJ'),
        ),
      ),
    );

    await tester.pumpAndSettle();

    expect(find.text('Rust DJ'), findsOneWidget);
    expect(find.text('Deck A'), findsWidgets);
    expect(find.text('Deck B'), findsWidgets);
    expect(find.text('Load tracks to see waveforms.'), findsOneWidget);
    expect(find.text('Collections'), findsOneWidget);
    expect(find.text('samples'), findsOneWidget);
    // First collection is selected by default.
    expect(find.text('Demo Track'), findsWidgets);
    expect(find.textContaining('Filter tracks'), findsOneWidget);
  });
}
