import 'dart:async';

import 'package:gui_flutter/shell/material_theme.dart';
import 'package:material_ui/material_ui.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:forui/forui.dart';
import 'package:gui_flutter/library/history_providers.dart';
import 'package:gui_flutter/library/providers.dart';
import 'package:gui_flutter/mixer/deck_panel.dart';
import 'package:gui_flutter/mixer/deck_tempo_panel.dart';
import 'package:gui_flutter/mixer/engine_providers.dart';
import 'package:gui_flutter/mixer/engine_ui.dart';
import 'package:gui_flutter/mixer/fader_slider.dart';
import 'package:gui_flutter/mixer/master_strip.dart';
import 'package:gui_flutter/mixer/mixer_strip.dart';
import 'package:gui_flutter/mixer/rotary_knob.dart';
import 'package:gui_flutter/shell/app_header.dart';
import 'package:gui_flutter/mixer/waveform/overview_strip.dart';
import 'package:gui_flutter/mixer/waveform/peaks.dart';
import 'package:gui_flutter/mixer/waveform/scrolling_lane.dart';
import 'package:gui_flutter/mixer/waveform/waveform_providers.dart';
import 'package:skeletonizer/skeletonizer.dart';
import 'package:gui_flutter/shell/app_shell.dart';
import 'package:gui_flutter/shell/desktop.dart';
import 'package:gui_flutter/shell/controller_providers.dart';
import 'package:gui_flutter/settings/settings_defaults.dart';
import 'package:gui_flutter/settings/settings_providers.dart';
import 'package:gui_flutter/settings/settings_page.dart';
import 'package:gui_flutter/src/rust/api/engine.dart';
import 'package:gui_flutter/src/rust/api/library.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';

class _HeaderCueEngineUi extends EngineUi {
  @override
  EngineUiSnapshot build() => const EngineUiSnapshot(
    running: true,
    titles: {},
    cueMix: 0.7,
    masterCue: true,
  );
}

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

class _LoadingDeckA extends DeckLoadInFlight {
  @override
  Map<int, int> build() => const {0: 1};
}

final _skeletonizerFinder = find.byWidgetPredicate((w) => w is Skeletonizer);

bool _enabledSkeletonsUnder(WidgetTester tester, Finder of) {
  return tester
      .widgetList<Skeletonizer>(
        find.descendant(of: of, matching: _skeletonizerFinder),
      )
      .any((s) => s.enabled);
}

// ignore: strict_top_level_inference
_settingsOverrides([AppSettings? settings]) => [
  appSettingsProvider.overrideWith(
    (ref) async => settings ?? defaultAppSettings(),
  ),
  audioDevicesProvider.overrideWith((ref, backend) async => const []),
  audioBackendNamesProvider.overrideWith(
    (ref) => const ['cpal', 'auto', 'null'],
  ),
  samplerBanksProvider.overrideWith((ref) async => const []),
  controllerTransportProvider.overrideWith((ref) async => null),
];

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  test('overlapping deck loads stay in flight until the last one finishes', () {
    final container = ProviderContainer();
    addTearDown(container.dispose);
    final loading = container.read(deckLoadInFlightProvider.notifier);
    loading.set(0, true);
    loading.set(0, true);
    expect(container.read(deckLoadingProvider(0)), isTrue);
    loading.set(0, false);
    expect(container.read(deckLoadingProvider(0)), isTrue);
    expect(container.read(deckLoadingProvider(1)), isFalse);
    loading.set(0, false);
    expect(container.read(deckLoadingProvider(0)), isFalse);
  });

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
    bool settle = true,
    AppSettings? settings,
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
          historySessionsProvider.overrideWith((ref) async => const []),
          historyCanResumeProvider.overrideWith((ref) async => false),
          ..._settingsOverrides(settings),
          ...extraOverrides,
        ],
        child: MaterialApp(
          theme: materialUiThemeFromForui(theme),
          builder: (context, child) => MaterialUiCompatibilityBridge(
            // ignore: deprecated_member_use
            child: FTheme(data: theme, child: child!),
          ),
          home: const AppShell(appTitle: 'Mixar'),
        ),
      ),
    );
    if (settle) {
      await tester.pumpAndSettle();
    } else {
      await tester.pump();
    }
  }

  testWidgets('mixer shell shows core regions', (tester) async {
    await pumpShell(tester);

    expect(find.bySemanticsLabel('Mixar'), findsOneWidget);
    expect(find.text('Deck A'), findsWidgets);
    expect(find.text('Deck B'), findsWidgets);
    expect(find.text('Load tracks to see waveforms.'), findsOneWidget);
    expect(find.text('COLLECTIONS'), findsOneWidget);
    expect(find.text('samples'), findsOneWidget);
    expect(find.text('Demo Track'), findsWidgets);
    expect(find.textContaining('Filter tracks'), findsOneWidget);
    expect(find.text('Engine idle'), findsOneWidget);
    expect(find.text('No track loaded'), findsWidgets);
    expect(find.text('Q'), findsWidgets);
    expect(find.text('Drop or load a track'), findsWidgets);
    expect(find.byKey(const ValueKey('mixer-panel-toggle')), findsOneWidget);
    expect(
      find.byKey(const ValueKey('crossfader-panel-toggle')),
      findsOneWidget,
    );
    expect(find.byKey(const ValueKey('master-panel-toggle')), findsOneWidget);
    expect(find.byType(MasterStrip), findsNothing);
    expect(
      find.descendant(
        of: find.byType(AppHeader),
        matching: find.text('Master'),
      ),
      findsNothing,
    );
    await tester.tap(find.byKey(const ValueKey('master-panel-toggle')));
    await tester.pumpAndSettle();
    expect(
      find.descendant(
        of: find.byType(MasterStrip),
        matching: find.text('CUE/MST'),
      ),
      findsOneWidget,
    );
    final masterCue = tester.widget<FButton>(
      find.descendant(
        of: find.byType(MasterStrip),
        matching: find.widgetWithText(FButton, 'Cue'),
      ),
    );
    expect(masterCue.onPress, isNull);
    final cueMix = tester.widget<RotaryKnob>(
      find.descendant(
        of: find.byType(MasterStrip),
        matching: find.byWidgetPredicate(
          (w) => w is RotaryKnob && w.label == 'Cue/Mst',
        ),
      ),
    );
    expect(cueMix.disabled, isTrue);
  });

  testWidgets('mixer cue controls enable with preview bus and follow status', (
    tester,
  ) async {
    await pumpShell(
      tester,
      extraOverrides: [engineUiProvider.overrideWith(_HeaderCueEngineUi.new)],
      settings: copyAppSettings(defaultAppSettings(), previewEnabled: true),
    );

    await tester.tap(find.byKey(const ValueKey('master-panel-toggle')));
    await tester.pumpAndSettle();

    final master = find.byType(MasterStrip);
    final masterCue = tester.widget<FButton>(
      find.descendant(
        of: master,
        matching: find.widgetWithText(FButton, 'Cue'),
      ),
    );
    expect(masterCue.onPress, isNotNull);
    expect(masterCue.variant == .secondary, isTrue);

    final cueMix = tester.widget<RotaryKnob>(
      find.descendant(
        of: master,
        matching: find.byWidgetPredicate(
          (w) => w is RotaryKnob && w.label == 'Cue/Mst',
        ),
      ),
    );
    expect(cueMix.disabled, isFalse);
    expect(cueMix.value, 0.7);
  });

  testWidgets('mixer section headers toggle their panels', (tester) async {
    await pumpShell(tester);
    final mixer = find.byKey(const ValueKey('mixer-panel-toggle'));
    final crossfader = find.byKey(const ValueKey('crossfader-panel-toggle'));
    final master = find.byKey(const ValueKey('master-panel-toggle'));
    Finder hiKnobs() =>
        find.byWidgetPredicate((w) => w is RotaryKnob && w.label == 'HI');
    Finder xfader() => find.byWidgetPredicate(
      (w) => w is FaderSlider && w.orientation == FaderOrientation.horizontal,
    );

    Future<void> press(Finder button) async {
      // Deck chrome can sit above the narrow mixer header in the fixed deck
      // row; drive the Forui button directly so the toggle still verifies.
      tester.widget<FButton>(button).onPress?.call();
      await tester.pumpAndSettle();
    }

    expect(hiKnobs(), findsWidgets);
    await press(mixer);
    expect(hiKnobs(), findsNothing);
    await press(mixer);
    expect(hiKnobs(), findsWidgets);

    expect(xfader(), findsOneWidget);
    await press(crossfader);
    expect(xfader(), findsNothing);
    await press(crossfader);
    expect(xfader(), findsOneWidget);

    expect(find.byType(MasterStrip), findsNothing);
    await press(master);
    expect(find.byType(MasterStrip), findsOneWidget);
    await press(master);
    expect(find.byType(MasterStrip), findsNothing);

    await press(mixer);
    await press(crossfader);
    expect(tester.getSize(find.byType(MixerStrip)).width, lessThan(212));
    await press(mixer);
    expect(tester.getSize(find.byType(MixerStrip)).width, 212);
  });

  testWidgets('settings switches waveform display mode', (tester) async {
    await pumpShell(tester);
    await tester.tap(find.byIcon(FLucideIcons.settings));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Waveform'));
    await tester.pumpAndSettle();
    expect(find.text('DISPLAY MODE'), findsOneWidget);
    expect(find.text('RGB'), findsOneWidget);
    expect(find.text('Save'), findsNothing);
  }, semanticsEnabled: false);

  testWidgets('settings page lists waveform and controllers', (tester) async {
    debugOverrideDesktopWindow = false;
    addTearDown(() => debugOverrideDesktopWindow = null);
    final theme = FTheme.neutral.light.desktop;
    tester.view.physicalSize = const Size(1400, 900);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);

    await tester.pumpWidget(
      ProviderScope(
        overrides: _settingsOverrides(),
        child: MaterialApp(
          theme: materialUiThemeFromForui(theme),
          builder: (context, child) => MaterialUiCompatibilityBridge(
            // ignore: deprecated_member_use
            child: FTheme(data: theme, child: child!),
          ),
          home: const SizedBox(width: 1400, height: 900, child: SettingsPage()),
        ),
      ),
    );
    await tester.pumpAndSettle();
    expect(find.text('Waveform'), findsWidgets);
    await tester.tap(find.text('Waveform'));
    await tester.pumpAndSettle();
    expect(find.text('DISPLAY MODE'), findsOneWidget);
    expect(find.text('Save'), findsNothing);

    await tester.tap(find.text('Audio'));
    await tester.pumpAndSettle();
    await tester.tap(find.byType(FSwitch).first);
    await tester.pumpAndSettle();
    expect(find.text('Save'), findsOneWidget);

    await tester.tap(find.text('Controllers'));
    await tester.pumpAndSettle();
    expect(find.text('Update All'), findsOneWidget);
  }, semanticsEnabled: false);

  testWidgets('deck shows loaded title from engine snapshot', (tester) async {
    await pumpShell(
      tester,
      extraOverrides: [
        engineUiProvider.overrideWith(_SeededEngineUi.new),
        waveformOverviewProvider.overrideWith(
          (ref, id) async => const <SpectralPeak>[],
        ),
        beatGridProvider.overrideWith((ref, id) => null),
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

  testWidgets('loading deck skeletons title, bpm, and overview preview', (
    tester,
  ) async {
    await pumpShell(
      tester,
      settle: false,
      extraOverrides: [
        deckLoadInFlightProvider.overrideWith(_LoadingDeckA.new),
      ],
    );

    final deckA = find.ancestor(
      of: find.descendant(
        of: find.byType(DeckPanel),
        matching: find.text('Deck A'),
      ),
      matching: find.byType(DeckPanel),
    );
    final deckB = find.ancestor(
      of: find.descendant(
        of: find.byType(DeckPanel),
        matching: find.text('Deck B'),
      ),
      matching: find.byType(DeckPanel),
    );
    final laneA = find.ancestor(
      of: find.descendant(
        of: find.byType(ScrollingLane),
        matching: find.text('Deck A'),
      ),
      matching: find.byType(ScrollingLane),
    );

    expect(_enabledSkeletonsUnder(tester, deckA), isTrue);
    expect(
      _enabledSkeletonsUnder(
        tester,
        find.descendant(of: deckA, matching: find.byType(OverviewStrip)),
      ),
      isTrue,
    );
    expect(
      _enabledSkeletonsUnder(
        tester,
        find.descendant(of: deckA, matching: find.byType(DeckTempoPanel)),
      ),
      isTrue,
    );
    expect(_enabledSkeletonsUnder(tester, laneA), isFalse);
    expect(_enabledSkeletonsUnder(tester, deckB), isFalse);
    expect(find.text('Load tracks to see waveforms.'), findsOneWidget);
  });

  testWidgets('loaded deck stays skeletonized while overview fetches', (
    tester,
  ) async {
    await pumpShell(
      tester,
      settle: false,
      extraOverrides: [
        engineUiProvider.overrideWith(_SeededEngineUi.new),
        waveformOverviewProvider.overrideWith(
          (ref, id) => Completer<List<SpectralPeak>>().future,
        ),
        beatGridProvider.overrideWith((ref, id) => null),
      ],
    );

    final deckA = find.ancestor(
      of: find.descendant(
        of: find.byType(DeckPanel),
        matching: find.text('Deck A'),
      ),
      matching: find.byType(DeckPanel),
    );
    expect(_enabledSkeletonsUnder(tester, deckA), isTrue);
    expect(find.text('Load tracks to see waveforms.'), findsNothing);
  });
}
