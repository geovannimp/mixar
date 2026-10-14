import 'dart:async';

import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_riverpod/misc.dart' show Override;
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/library/history_providers.dart';
import 'package:gui_flutter/library/providers.dart';
import 'package:gui_flutter/mixer/deck_panel.dart';
import 'package:gui_flutter/mixer/deck_tempo_panel.dart';
import 'package:gui_flutter/mixer/engine_providers.dart';
import 'package:gui_flutter/mixer/engine_ui.dart';
import 'package:gui_flutter/mixer/fader_slider.dart';
import 'package:gui_flutter/mixer/master_strip.dart';
import 'package:gui_flutter/mixer/mixer_button.dart';
import 'package:gui_flutter/mixer/mixer_strip.dart';
import 'package:gui_flutter/mixer/rotary_knob.dart';
import 'package:gui_flutter/mixer/waveform/overview_strip.dart';
import 'package:gui_flutter/mixer/waveform/peaks.dart';
import 'package:gui_flutter/mixer/waveform/scrolling_lane.dart';
import 'package:gui_flutter/mixer/waveform/waveform_providers.dart';
import 'package:gui_flutter/settings/settings_audio_panel.dart';
import 'package:gui_flutter/settings/settings_defaults.dart';
import 'package:gui_flutter/settings/settings_page.dart';
import 'package:gui_flutter/settings/settings_providers.dart';
import 'package:gui_flutter/shell/app_button.dart';
import 'package:gui_flutter/shell/app_header.dart';
import 'package:gui_flutter/shell/app_shell.dart';
import 'package:gui_flutter/shell/controller_providers.dart';
import 'package:gui_flutter/shell/desktop.dart';
import 'package:gui_flutter/shell/mixar_switch.dart';
import 'package:gui_flutter/shell/mixar_theme.dart';
import 'package:gui_flutter/src/rust/api/engine.dart';
import 'package:gui_flutter/src/rust/api/library.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';
import 'package:skeletonizer/skeletonizer.dart';

import 'helpers/collection_tracks_override.dart';
import 'support/fake_engine_transport.dart';
import 'support/mixar_material_app.dart';

class _HeaderCueEngineUi extends EngineUi {
  @override
  EngineUiSnapshot build() => const EngineUiSnapshot(
    running: true,
    trackPaths: {},
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
      trackPath: '/seeded.flac',
      trackId: 't1',
      durationMs: 180000,
    ),
  );
}

class _LoadingDeckA extends DeckLoadInFlight {
  @override
  Map<int, int> build() => const {0: 1};
}

final Finder _skeletonizerFinder = find.byWidgetPredicate(
  (w) => w is Skeletonizer,
);

bool _enabledSkeletonsUnder(WidgetTester tester, Finder of) {
  return tester
      .widgetList<Skeletonizer>(
        find.descendant(of: of, matching: _skeletonizerFinder),
      )
      .any((s) => s.enabled);
}

List<Override> _settingsOverrides([AppSettings? settings]) => [
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
    bpm: 128,
    key: '8A',
    durationMs: 180000,
    path: '/tmp/samples/demo.wav',
  );

  Future<void> pumpShell(
    WidgetTester tester, {
    List<Override> extraOverrides = const [],
    bool settle = true,
    AppSettings? settings,
  }) async {
    debugOverrideDesktopWindow = false;
    addTearDown(() => debugOverrideDesktopWindow = null);

    final theme = MixarThemeData.light();
    tester.view.physicalSize = const Size(1400, 900);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);

    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          collectionsProvider.overrideWith((ref) async => [collection]),
          overrideCollectionTracks((ref) async {
            final id = ref.watch(activeCollectionIdProvider);
            return id == collection.id ? [track] : const [];
          }),
          historySessionsProvider.overrideWith((ref) async => const []),
          historyCanResumeProvider.overrideWith((ref) async => false),
          ..._settingsOverrides(settings),
          ...extraOverrides,
        ],
        child: mixarTestMaterialApp(
          theme: theme,
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
    expect(find.text('DECK A'), findsWidgets);
    expect(find.text('DECK B'), findsWidgets);
    expect(find.text('Load tracks to see waveforms.'), findsOneWidget);
    expect(find.text('COLLECTIONS'), findsOneWidget);
    expect(find.text('samples'), findsOneWidget);
    expect(find.text('Demo Track'), findsWidgets);
    expect(find.textContaining('Filter tracks'), findsOneWidget);
    expect(find.text('Engine idle'), findsNothing);
    expect(find.text('Engine running'), findsNothing);
    expect(find.text('No track loaded'), findsWidgets);
    expect(find.text('Q'), findsWidgets);
    expect(find.text('Drop or load a track'), findsWidgets);
    expect(find.byKey(const ValueKey('mixer-panel-toggle')), findsOneWidget);
    expect(
      find.byKey(const ValueKey('crossfader-panel-toggle')),
      findsOneWidget,
    );
    expect(find.byKey(const ValueKey('master-panel-toggle')), findsNothing);
    expect(
      find.descendant(
        of: find.byType(AppHeader),
        matching: find.byKey(const ValueKey('master-meter')),
      ),
      findsOneWidget,
    );
    expect(find.byType(MasterStrip), findsNothing);
    await tester.tap(find.byKey(const ValueKey('master-meter')));
    await tester.pumpAndSettle();
    expect(
      find.descendant(
        of: find.byType(MasterStrip),
        matching: find.text('Levels'),
      ),
      findsOneWidget,
    );
    expect(
      find.descendant(
        of: find.byType(MasterStrip),
        matching: find.text('Master'),
      ),
      findsOneWidget,
    );
    expect(
      find.descendant(
        of: find.byType(MasterStrip),
        matching: find.text('Headphone'),
      ),
      findsOneWidget,
    );
    expect(
      find.descendant(
        of: find.byType(MasterStrip),
        matching: find.widgetWithText(AppButton, 'Enable'),
      ),
      findsOneWidget,
    );
    expect(
      find.descendant(
        of: find.byType(MasterStrip),
        matching: find.text('Mixing'),
      ),
      findsOneWidget,
    );
    expect(
      find.descendant(of: find.byType(MasterStrip), matching: find.text('Cue')),
      findsOneWidget,
    );
    expect(
      find.descendant(of: find.byType(MasterStrip), matching: find.text('Mst')),
      findsOneWidget,
    );
    final masterCue = tester.widget<MixerButton>(
      find.descendant(
        of: find.byType(MasterStrip),
        matching: find.byWidgetPredicate(
          (w) => w is MixerButton && w.semanticsLabel == 'Master cue',
        ),
      ),
    );
    expect(masterCue.onPress, isNotNull);
    expect(
      find.descendant(
        of: find.byType(MasterStrip),
        matching: find.byIcon(LucideIcons.headphones),
      ),
      findsOneWidget,
    );
    final masterLevel = tester.widget<FaderSlider>(
      find.descendant(
        of: find.byType(MasterStrip),
        matching: find.byWidgetPredicate(
          (w) => w is FaderSlider && w.semanticLabel == 'Master level',
        ),
      ),
    );
    expect(masterLevel.value, 100);
    expect(masterLevel.disabled, isFalse);
    expect(masterLevel.showIndicator, isTrue);
    expect(masterLevel.orientation, FaderOrientation.horizontal);
    final headphoneLevel = tester.widget<FaderSlider>(
      find.descendant(
        of: find.byType(MasterStrip),
        matching: find.byWidgetPredicate(
          (w) => w is FaderSlider && w.semanticLabel == 'Headphone level',
        ),
      ),
    );
    expect(headphoneLevel.value, 100);
    expect(headphoneLevel.disabled, isFalse);
    expect(headphoneLevel.showIndicator, isTrue);
    final cueMix = tester.widget<FaderSlider>(
      find.descendant(
        of: find.byType(MasterStrip),
        matching: find.byWidgetPredicate(
          (w) => w is FaderSlider && w.semanticLabel == 'Cue/Mst',
        ),
      ),
    );
    expect(cueMix.disabled, isFalse);
    expect(cueMix.centerNotch, isTrue);
    expect(cueMix.value, 50);
    expect(cueMix.orientation, FaderOrientation.horizontal);
    expect(
      find.descendant(
        of: find.byType(MasterStrip),
        matching: find.widgetWithText(MixerButton, 'M'),
      ),
      findsNothing,
    );

    await tester.tap(
      find.descendant(
        of: find.byType(MasterStrip),
        matching: find.widgetWithText(AppButton, 'Enable'),
      ),
    );
    await tester.pumpAndSettle();
    expect(find.byType(MasterStrip), findsNothing);
    expect(find.byType(SettingsPage), findsOneWidget);
    expect(find.text('Preview bus'), findsOneWidget);
    expect(find.byKey(SettingsAudioPanel.previewBusCardKey), findsOneWidget);
    final previewRect = tester.getRect(
      find.byKey(SettingsAudioPanel.previewBusCardKey),
    );
    final viewSize = tester.view.physicalSize / tester.view.devicePixelRatio;
    expect(previewRect.top, lessThan(viewSize.height));
    expect(previewRect.bottom, greaterThan(0));
  });

  testWidgets('mixer cue controls enable with preview bus and follow status', (
    tester,
  ) async {
    await pumpShell(
      tester,
      extraOverrides: [engineUiProvider.overrideWith(_HeaderCueEngineUi.new)],
      settings: copyAppSettings(defaultAppSettings(), previewEnabled: true),
    );

    await tester.tap(find.byKey(const ValueKey('master-meter')));
    await tester.pumpAndSettle();

    final master = find.byType(MasterStrip);
    final masterCue = tester.widget<MixerButton>(
      find.descendant(
        of: master,
        matching: find.byWidgetPredicate(
          (w) => w is MixerButton && w.semanticsLabel == 'Master cue',
        ),
      ),
    );
    expect(masterCue.onPress, isNotNull);
    expect(masterCue.variant == .secondary, isTrue);
    expect(
      find.descendant(
        of: master,
        matching: find.byIcon(LucideIcons.headphones),
      ),
      findsOneWidget,
    );

    final masterLevel = tester.widget<FaderSlider>(
      find.descendant(
        of: master,
        matching: find.byWidgetPredicate(
          (w) => w is FaderSlider && w.semanticLabel == 'Master level',
        ),
      ),
    );
    expect(masterLevel.value, 100);
    expect(masterLevel.showIndicator, isTrue);

    final headphoneLevel = tester.widget<FaderSlider>(
      find.descendant(
        of: master,
        matching: find.byWidgetPredicate(
          (w) => w is FaderSlider && w.semanticLabel == 'Headphone level',
        ),
      ),
    );
    expect(headphoneLevel.disabled, isFalse);
    expect(headphoneLevel.value, 100);
    expect(
      find.descendant(
        of: master,
        matching: find.widgetWithText(AppButton, 'Enable'),
      ),
      findsNothing,
    );
    expect(
      find.descendant(
        of: master,
        matching: find.widgetWithText(AppButton, 'On'),
      ),
      findsNothing,
    );

    expect(
      find.descendant(of: master, matching: find.text('Mixing')),
      findsOneWidget,
    );

    final cueMix = tester.widget<FaderSlider>(
      find.descendant(
        of: master,
        matching: find.byWidgetPredicate(
          (w) => w is FaderSlider && w.semanticLabel == 'Cue/Mst',
        ),
      ),
    );
    expect(cueMix.disabled, isFalse);
    expect(cueMix.value, 70);
    expect(cueMix.centerNotch, isTrue);
  });

  testWidgets('Levels faders call master/cue volume transport', (tester) async {
    final engine = FakeEngineTransport();
    await pumpShell(
      tester,
      extraOverrides: [
        engineTransportProvider.overrideWith((ref) async => engine),
      ],
    );

    await tester.tap(find.byKey(const ValueKey('master-meter')));
    await tester.pumpAndSettle();

    Finder fader(String label) => find.descendant(
      of: find.byType(MasterStrip),
      matching: find.byWidgetPredicate(
        (w) => w is FaderSlider && w.semanticLabel == label,
      ),
    );

    await tester.drag(fader('Master level'), const Offset(-40, 0));
    await tester.pumpAndSettle();
    expect(engine.masterVolumes, isNotEmpty);
    expect(engine.masterVolumes.last, lessThan(1.0));
    expect(engine.masterVolumes.last, greaterThanOrEqualTo(0.0));

    await tester.drag(fader('Headphone level'), const Offset(-40, 0));
    await tester.pumpAndSettle();
    expect(engine.cueVolumes, isNotEmpty);
    expect(engine.cueVolumes.last, lessThan(1.0));

    await tester.drag(fader('Cue/Mst'), const Offset(30, 0));
    await tester.pumpAndSettle();
    expect(engine.cueMixes, isNotEmpty);
    expect(engine.cueMixes.last, inInclusiveRange(0.0, 1.0));
  });

  testWidgets('mixer section headers toggle their panels', (tester) async {
    await pumpShell(tester);
    final mixer = find.byKey(const ValueKey('mixer-panel-toggle'));
    final crossfader = find.byKey(const ValueKey('crossfader-panel-toggle'));
    Finder hiKnobs() =>
        find.byWidgetPredicate((w) => w is RotaryKnob && w.label == 'HI');
    Finder xfader() => find.byWidgetPredicate(
      (w) => w is FaderSlider && w.orientation == FaderOrientation.horizontal,
    );

    Future<void> press(Finder button) async {
      // Deck chrome can sit above the narrow mixer header in the fixed deck
      // row; drive the MixerButton directly so the toggle still verifies.
      tester.widget<MixerButton>(button).onPress?.call();
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

    expect(find.byKey(const ValueKey('master-panel-toggle')), findsNothing);

    await press(mixer);
    await press(crossfader);
    expect(tester.getSize(find.byType(MixerStrip)).width, lessThan(212));
    await press(mixer);
    expect(tester.getSize(find.byType(MixerStrip)).width, 212);
  });

  testWidgets('hiding mixer meters narrows the strip', (tester) async {
    await pumpShell(
      tester,
      settings: copyAppSettings(defaultAppSettings(), showMixerMeters: false),
    );
    expect(tester.getSize(find.byType(MixerStrip)).width, 168);
    expect(find.byKey(const ValueKey('master-meter')), findsOneWidget);
  });

  testWidgets('settings switches waveform display mode', (tester) async {
    await pumpShell(tester);
    await tester.tap(find.byIcon(LucideIcons.settings));
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
    final theme = MixarThemeData.light();
    tester.view.physicalSize = const Size(1400, 900);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);

    await tester.pumpWidget(
      ProviderScope(
        overrides: _settingsOverrides(),
        child: mixarTestMaterialApp(
          theme: theme,
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
    await tester.tap(find.byType(MixarSwitch).first);
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
        beatGridFetchProvider.overrideWith((ref, id) async => null),
      ],
    );

    expect(
      find.descendant(
        of: find.byType(DeckPanel),
        matching: find.text('Demo Track'),
      ),
      findsOneWidget,
    );
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
        matching: find.text('DECK A'),
      ),
      matching: find.byType(DeckPanel),
    );
    final deckB = find.ancestor(
      of: find.descendant(
        of: find.byType(DeckPanel),
        matching: find.text('DECK B'),
      ),
      matching: find.byType(DeckPanel),
    );
    final laneA = find.ancestor(
      of: find.descendant(
        of: find.byType(ScrollingLane),
        matching: find.text('A'),
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
        beatGridFetchProvider.overrideWith((ref, id) async => null),
      ],
    );

    final deckA = find.ancestor(
      of: find.descendant(
        of: find.byType(DeckPanel),
        matching: find.text('DECK A'),
      ),
      matching: find.byType(DeckPanel),
    );
    expect(_enabledSkeletonsUnder(tester, deckA), isTrue);
    expect(find.text('Load tracks to see waveforms.'), findsNothing);
  });
}
