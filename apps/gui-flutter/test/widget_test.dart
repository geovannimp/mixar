import 'package:gui_flutter/shell/material_theme.dart';
import 'package:material_ui/material_ui.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:forui/forui.dart';
import 'package:gui_flutter/library/providers.dart';
import 'package:gui_flutter/mixer/deck_panel.dart';
import 'package:gui_flutter/mixer/engine_providers.dart';
import 'package:gui_flutter/mixer/engine_ui.dart';
import 'package:gui_flutter/mixer/waveform/overview_strip.dart';
import 'package:gui_flutter/mixer/waveform/peaks.dart';
import 'package:gui_flutter/mixer/waveform/spectral_color.dart';
import 'package:gui_flutter/mixer/waveform/waveform_providers.dart';
import 'package:gui_flutter/shell/app_shell.dart';
import 'package:gui_flutter/shell/desktop.dart';
import 'package:gui_flutter/src/rust/api/engine.dart';
import 'package:gui_flutter/src/rust/api/library.dart';

class _SeededEngineUi extends EngineUi {
  @override
  EngineUiSnapshot build() => applyEngineEvt(
    EngineUiSnapshot.empty,
    const EngineEvt(
      kind: EngineEvtKind.updated,
      deckId: 0,
      track: 'Seeded Track',
      trackId: 't1',
      durationMs: 180000,
    ),
  );
}

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

  Future<void> pumpShell(
    WidgetTester tester, {
    List extraOverrides = const [],
  }) async {
    debugOverrideDesktopWindow = false;
    addTearDown(() => debugOverrideDesktopWindow = null);

    final theme = FTheme.neutral.light.desktop;
    tester.view.physicalSize = const Size(1400, 900);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);

    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          collectionsProvider.overrideWith((ref) async => [collection]),
          collectionTracksProvider.overrideWith((ref) async {
            final id = ref.watch(activeCollectionIdProvider);
            return id == collection.id ? [track] : const [];
          }),
          ...extraOverrides,
        ],
        child: MaterialApp(
          theme: materialUiThemeFromForui(theme),
          builder: (context, child) => MaterialUiCompatibilityBridge(
            // ignore: deprecated_member_use
            child: FTheme(data: theme, child: child!),
          ),
          home: const AppShell(appTitle: 'Rust DJ'),
        ),
      ),
    );
    await tester.pumpAndSettle();
  }

  testWidgets('mixer shell shows core regions', (tester) async {
    await pumpShell(tester);

    expect(find.text('Rust DJ'), findsOneWidget);
    expect(find.text('Deck A'), findsWidgets);
    expect(find.text('Deck B'), findsWidgets);
    expect(find.text('Load tracks to see waveforms.'), findsOneWidget);
    expect(find.text('Collections'), findsOneWidget);
    expect(find.text('samples'), findsOneWidget);
    expect(find.text('Demo Track'), findsWidgets);
    expect(find.textContaining('Filter tracks'), findsOneWidget);
    expect(find.text('Engine idle'), findsOneWidget);
    expect(find.text('No track loaded'), findsWidgets);
  });

  testWidgets('settings switches waveform display mode', (tester) async {
    await pumpShell(tester);
    final container = ProviderScope.containerOf(
      tester.element(find.byType(AppShell)),
    );
    expect(container.read(waveformDisplayModeProvider), WaveformDisplayMode.rgb);

    await tester.tap(find.text('Settings'));
    await tester.pumpAndSettle();
    expect(find.text('Waveform'), findsOneWidget);

    await tester.tap(find.text('Filtered'));
    await tester.pumpAndSettle();
    expect(
      container.read(waveformDisplayModeProvider),
      WaveformDisplayMode.filtered,
    );

    await tester.tap(find.text('RGB'));
    await tester.pumpAndSettle();
    expect(container.read(waveformDisplayModeProvider), WaveformDisplayMode.rgb);
  });

  testWidgets('deck shows loaded title from engine snapshot', (tester) async {
    await pumpShell(
      tester,
      extraOverrides: [
        engineUiProvider.overrideWith(_SeededEngineUi.new),
        waveformOverviewProvider.overrideWith(
          (ref, id) async => const <SpectralPeak>[],
        ),
        beatGridProvider.overrideWith((ref, id) async => null),
      ],
    );

    expect(find.text('Seeded Track'), findsOneWidget);
    expect(find.text('Load tracks to see waveforms.'), findsNothing);
    expect(
      find.descendant(
        of: find.byType(DeckPanel),
        matching: find.byType(OverviewStrip),
      ),
      findsNWidgets(2),
    );
  });
}
