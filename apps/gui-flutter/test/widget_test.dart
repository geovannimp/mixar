import 'dart:async';

import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_riverpod/misc.dart' show Override;
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/models/bus_enums.dart';
import 'package:gui_flutter/core/models/engine_ui.dart';
import 'package:gui_flutter/core/models/settings_section.dart';
import 'package:gui_flutter/core/models/waveform_peaks.dart';
import 'package:gui_flutter/core/utils/desktop.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/providers/library/drawer_providers.dart';
import 'package:gui_flutter/providers/library/history_providers.dart';
import 'package:gui_flutter/providers/library/providers.dart';
import 'package:gui_flutter/providers/mixer/engine_providers.dart';
import 'package:gui_flutter/providers/mixer/waveform/waveform_providers.dart';
import 'package:gui_flutter/providers/settings/settings_providers.dart';
import 'package:gui_flutter/providers/shell/controller_providers.dart';
import 'package:gui_flutter/screens/app_shell.dart';
import 'package:gui_flutter/screens/settings_page.dart';
import 'package:gui_flutter/src/rust/api/library.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';
import 'package:gui_flutter/widgets/mixer/deck_cover.dart';
import 'package:gui_flutter/widgets/mixer/deck_panel.dart';
import 'package:gui_flutter/widgets/mixer/deck_tempo_panel.dart';
import 'package:gui_flutter/widgets/mixer/fader_slider.dart';
import 'package:gui_flutter/widgets/mixer/fx_bar.dart';
import 'package:gui_flutter/widgets/mixer/layout/mixer_slot.dart';
import 'package:gui_flutter/widgets/mixer/library_panel.dart';
import 'package:gui_flutter/widgets/mixer/master_strip.dart';
import 'package:gui_flutter/widgets/mixer/mixer_button.dart';
import 'package:gui_flutter/widgets/mixer/mixer_strip.dart';
import 'package:gui_flutter/widgets/mixer/pads/loop_roll_pads.dart';
import 'package:gui_flutter/widgets/mixer/pads/pad_button.dart';
import 'package:gui_flutter/widgets/mixer/rotary_knob.dart';
import 'package:gui_flutter/widgets/mixer/waveform/overview_strip.dart';
import 'package:gui_flutter/widgets/mixer/waveform/scrolling_lane.dart';
import 'package:gui_flutter/widgets/mixer/waveform/vertical_lane.dart';
import 'package:gui_flutter/widgets/mixer/waveform_section.dart';
import 'package:gui_flutter/widgets/shell/app_button.dart';
import 'package:gui_flutter/widgets/shell/app_header.dart';
import 'package:gui_flutter/widgets/shell/mixar_select.dart';
import 'package:gui_flutter/widgets/shell/mixar_switch.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';
import 'package:skeletonizer/skeletonizer.dart';

import 'helpers/collection_tracks_override.dart';

import 'package:gui_flutter/widgets/settings/settings_glow_flash.dart';

import 'support/bus_events.dart';
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

class _RunningEmptyEngineUi extends EngineUi {
  @override
  EngineUiSnapshot build() =>
      const EngineUiSnapshot(running: true, trackPaths: {});
}

class _SeededEngineUi extends EngineUi {
  @override
  EngineUiSnapshot build() => applyEngineEvent(
    EngineUiSnapshot.empty,
    deckUpdatedEvent(
      deck: 0,
      update: deckUpdated(
        deck: 0,
        trackPath: '/seeded.flac',
        trackId: 't1',
        durationMs: 180000,
      ),
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

Finder _deck(String label) => find.ancestor(
  of: find.descendant(of: find.byType(DeckPanel), matching: find.text(label)),
  matching: find.byType(DeckPanel),
);

List<Override> _settingsOverrides([AppSettings? settings]) => [
  appSettingsProvider.overrideWith(
    (ref) async => settings ?? defaultAppSettings(),
  ),
  audioDevicesProvider.overrideWith((ref) async => const []),
  samplerBanksProvider.overrideWith((ref) async => const []),
  controllerTransportProvider.overrideWith((ref) async => null),
];

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  // The deck row is the mixer's content slot; its key replaces `DeckGrid` as
  // the handle on the row's rectangle.
  final mixerContentRow = find.byKey(const ValueKey('mixer-content-row'));

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
    Size size = const Size(1400, 900),
    double devicePixelRatio = 1,
    AppSettings? settings,
    MixarThemeData? theme,
  }) async {
    debugOverrideDesktopWindow = false;
    addTearDown(() => debugOverrideDesktopWindow = null);

    final activeTheme = theme ?? MixarThemeData.light();
    tester.view.physicalSize = size;
    tester.view.devicePixelRatio = devicePixelRatio;
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
          theme: activeTheme,
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

  testWidgets('four-deck profile renders all four deck panels', (tester) async {
    await pumpShell(
      tester,
      settings: copyAppSettings(defaultAppSettings(), numDecks: 4),
    );
    expect(find.byType(DeckPanel), findsNWidgets(4));
    for (final label in ['DECK A', 'DECK B', 'DECK C', 'DECK D']) {
      expect(_deck(label), findsOneWidget);
    }
    expect(tester.getSize(find.byType(LibraryPanel)).height, greaterThan(0));
    expect(tester.takeException(), isNull);
  });

  testWidgets('four decks send the library to the drawer at 1024x768', (
    tester,
  ) async {
    await pumpShell(
      tester,
      size: const Size(1024, 768),
      settings: copyAppSettings(defaultAppSettings(), numDecks: 4),
    );
    expect(tester.takeException(), isNull);
    // The library cannot dock at this size, so it leaves the body entirely.
    expect(find.byType(LibraryPanel), findsNothing);
    final container = ProviderScope.containerOf(
      tester.element(find.byType(AppShell)),
    );
    expect(container.read(mixerLibraryForcedDrawerProvider), isTrue);
    // The decks take its room: only the band and the FX bar keep theirs.
    final content = tester.getRect(mixerContentRow);
    expect(tester.getRect(find.byType(WaveformSection)).bottom, content.top);
    expect(tester.getRect(find.byType(FxBar)).top, content.bottom);
    expect(content.height, greaterThan(500));
    expect(
      Scrollable.maybeOf(tester.element(mixerContentRow)),
      isNull,
      reason: 'the body must not scroll',
    );
    expect(tester.takeException(), isNull);
  });

  testWidgets('the deck cover opens the drawer a short window forces', (
    tester,
  ) async {
    await pumpShell(
      tester,
      size: const Size(1024, 600),
      settings: copyAppSettings(defaultAppSettings(), numDecks: 4),
    );
    expect(find.byType(LibraryPanel), findsNothing);
    final container = ProviderScope.containerOf(
      tester.element(find.byType(AppShell)),
    );
    expect(container.read(mixerLibraryForcedDrawerProvider), isTrue);

    await tester.tap(find.byType(DeckCover).first);
    await tester.pumpAndSettle();

    expect(
      find.byType(LibraryPanel),
      findsOneWidget,
      reason: 'the cover must reach a library that has no docked pane',
    );
    expect(tester.takeException(), isNull);
  });

  testWidgets('pad scrolling stays isolated from library drag scrolling', (
    tester,
  ) async {
    await pumpShell(tester, size: const Size(1024, 768));
    expect(
      ScrollConfiguration.of(tester.element(find.byType(PadButton).first))
          .dragDevices,
      isEmpty,
      reason: 'a press-drag on a pad must never become a scroll',
    );
    expect(
      Scrollable.maybeOf(tester.element(mixerContentRow)),
      isNull,
      reason: 'the body does not scroll, so a pad press has nothing to drag',
    );
    expect(
      ScrollConfiguration.of(tester.element(find.byType(LibraryPanel)))
          .dragDevices,
      isNotEmpty,
      reason: 'docked library lists keep their default drag scrolling',
    );
  });

  testWidgets('four-deck drawer mode expands the deck grid without overflow', (
    tester,
  ) async {
    await pumpShell(
      tester,
      settings: copyAppSettings(
        defaultAppSettings(),
        numDecks: 4,
        libraryDrawerEnabled: true,
      ),
    );

    expect(find.byType(DeckPanel), findsNWidgets(4));
    expect(tester.takeException(), isNull);
  });

  testWidgets('empty deck can switch pad mode', (tester) async {
    final engine = FakeEngineTransport();
    await pumpShell(
      tester,
      extraOverrides: [
        engineUiProvider.overrideWith(_RunningEmptyEngineUi.new),
        engineTransportProvider.overrideWith((ref) async => engine),
      ],
    );

    final deckA = _deck('DECK A');
    final padModeSelect = find.descendant(
      of: deckA,
      matching: find.byType(MixarSelect<PadMode>),
    );
    expect(padModeSelect, findsOneWidget);

    await tester.tap(padModeSelect);
    await tester.pumpAndSettle();
    await tester.tap(find.text('Roll').last);
    await tester.pumpAndSettle();

    expect(engine.padModes, [(0, PadMode.loopRoll)]);

    // The engine echoes the new mode back as a full deck snapshot; there is no
    // fake bus, so the test pushes the event the shell would receive.
    ProviderScope.containerOf(tester.element(find.byType(AppShell)))
        .read(engineUiProvider.notifier)
        .apply(
          deckUpdatedEvent(
            deck: 0,
            update: deckUpdated(deck: 0, padMode: PadMode.loopRoll),
          ),
        );
    await tester.pumpAndSettle();

    // The selector works on an empty deck, but its pads stay inert.
    await tester.tap(
      find.descendant(
        of: find.byType(LoopRollPads),
        matching: find.text('1/2'),
      ),
    );
    await tester.pumpAndSettle();
    expect(engine.loopRollPresses, isEmpty);
  });

  testWidgets('leaving the mixer closes the library drawer', (tester) async {
    await pumpShell(
      tester,
      settings: copyAppSettings(
        defaultAppSettings(),
        libraryDrawerEnabled: true,
      ),
    );

    final container = ProviderScope.containerOf(
      tester.element(find.byType(AppShell)),
    );

    // The header opens the drawer...
    await tester.tap(
      find.descendant(
        of: find.byType(AppHeader),
        matching: find.byIcon(LucideIcons.libraryBig),
      ),
    );
    await tester.pumpAndSettle();
    expect(find.byType(LibraryPanel), findsOneWidget);

    // ...and leaving the mixer puts it away, so it cannot sit over settings.
    await tester.tap(
      find.descendant(
        of: find.byType(AppHeader),
        matching: find.byIcon(LucideIcons.settings),
      ),
    );
    await tester.pumpAndSettle();
    expect(find.byType(LibraryPanel), findsNothing);
    expect(container.read(libraryDrawerOpenProvider), isFalse);
  });

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
    expect(find.text('Drop a track or click the cover'), findsWidgets);
    expect(find.byKey(const ValueKey('mixer-panel-toggle')), findsNothing);
    expect(find.byKey(const ValueKey('crossfader-panel-toggle')), findsNothing);
    expect(find.text('Mixer'), findsOneWidget);
    expect(find.byKey(const ValueKey('master-panel-toggle')), findsNothing);
    expect(
      find.descendant(
        of: find.byType(FxBar),
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
  });

  testWidgets('Levels Enable deep-links to Preview bus settings', (
    tester,
  ) async {
    await pumpShell(tester);

    await tester.tap(find.byKey(const ValueKey('master-meter')));
    await tester.pumpAndSettle();
    expect(
      find.descendant(
        of: find.byType(MasterStrip),
        matching: find.widgetWithText(AppButton, 'Enable'),
      ),
      findsOneWidget,
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
    // Overlay lifecycle is covered in settings_glow_flash_test; here we only
    // assert the deep-link armed the one-shot flash.
    expect(
      tester.widget<SettingsGlowFlash>(find.byType(SettingsGlowFlash)).active,
      isTrue,
    );
  });

  testWidgets('SettingsPage scrollToPreviewBus brings Preview bus on-screen', (
    tester,
  ) async {
    debugOverrideDesktopWindow = false;
    addTearDown(() => debugOverrideDesktopWindow = null);
    final theme = MixarThemeData.dark();
    // Short enough that Preview starts below the fold; tall enough for sidebar.
    tester.view.physicalSize = const Size(900, 520);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);

    await tester.pumpWidget(
      ProviderScope(
        overrides: _settingsOverrides(),
        child: mixarTestMaterialApp(
          theme: theme,
          home: const SizedBox(
            width: 900,
            height: 520,
            child: SettingsPage(
              initialSection: SettingsSection.audio,
              scrollToPreviewBus: true,
            ),
          ),
        ),
      ),
    );
    // First data paint mounts the card off-screen; scroll runs next frame.
    await tester.pump();
    expect(find.text('Preview bus'), findsOneWidget);
    final beforeTop = tester.getRect(find.text('Preview bus')).top;
    expect(beforeTop, greaterThanOrEqualTo(520));

    await tester.pumpAndSettle();
    final afterTop = tester.getRect(find.text('Preview bus')).top;
    expect(afterTop, lessThan(520));
    expect(find.text('Preview bus').hitTestable(), findsOneWidget);
    expect(
      tester.widget<SettingsGlowFlash>(find.byType(SettingsGlowFlash)).active,
      isTrue,
    );
  }, semanticsEnabled: false);

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

  testWidgets('crossfader rule bleeds the deck gutter; row is centred', (
    tester,
  ) async {
    await pumpShell(tester);
    Finder hiKnobs() =>
        find.byWidgetPredicate((w) => w is RotaryKnob && w.label == 'HI');
    Finder xfader() => find.byWidgetPredicate(
      (w) => w is FaderSlider && w.orientation == FaderOrientation.horizontal,
    );

    expect(hiKnobs(), findsWidgets);
    expect(xfader(), findsOneWidget);

    // The mixer slot frame keeps the strip's gutter on both sides; the rule
    // bleeds into it so it reaches the deck card borders.
    const gutter = MixerPaneMetrics.mixerBleed;
    final stripWidth = tester.getSize(find.byType(MixerStrip)).width;
    final rule = find.byKey(const ValueKey('crossfader-rule'));
    expect(tester.getSize(rule).width, stripWidth + 2 * gutter);
    expect(
      tester.getTopLeft(rule).dx,
      tester.getTopLeft(find.byType(MixerStrip)).dx - gutter,
    );

    // The crossfader row sits centred between the rule and the panel bottom.
    final ruleBottom = tester.getRect(rule).bottom;
    final rowCentre = tester.getRect(xfader()).center.dy;
    final panelBottom = tester.getRect(find.byType(MixerStrip)).bottom;
    expect(rowCentre - ruleBottom, closeTo(panelBottom - rowCentre, 0.5));
  });

  testWidgets('hiding mixer meters narrows the strip', (tester) async {
    await pumpShell(
      tester,
      settings: copyAppSettings(
        defaultAppSettings(),
        mixerMeters: MixerMetersSetting.none,
      ),
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

  testWidgets('settings opens on the UI section', (tester) async {
    await pumpShell(tester);
    await tester.tap(find.byIcon(LucideIcons.settings));
    await tester.pumpAndSettle();

    expect(find.byType(SettingsPage), findsOneWidget);
    expect(
      find.text('Brightness and the accent color used by brand chrome.'),
      findsOneWidget,
      reason: 'the UI section is the default landing tab',
    );
    expect(find.text('SAMPLE RATE'), findsNothing);
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
    expect(find.text('BACKEND'), findsNothing);
    expect(find.text('SAMPLE RATE'), findsOneWidget);
    expect(find.text('Low latency'), findsOneWidget);
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

    final deckA = _deck('DECK A');
    final deckB = _deck('DECK B');
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

    final deckA = _deck('DECK A');
    expect(_enabledSkeletonsUnder(tester, deckA), isTrue);
    expect(find.text('Load tracks to see waveforms.'), findsNothing);
  });

  testWidgets('tempo rail runs from the waveform pane to the library divider', (
    tester,
  ) async {
    await pumpShell(tester);

    final rail = find.descendant(
      of: _deck('DECK A'),
      matching: find.byType(DeckTempoPanel),
    );

    expect(
      tester.getTopLeft(rail).dy,
      tester.getBottomLeft(find.byType(WaveformSection)).dy,
      reason: 'the rail has no card top edge: it meets the waveform pane',
    );
    expect(
      tester.getBottomLeft(rail).dy,
      tester.getTopLeft(find.byType(LibraryPanel)).dy,
      reason: 'the rail meets the divider above the docked library',
    );
    expect(tester.takeException(), isNull);
  });

  testWidgets('header foot hairline caps the waveform pane', (tester) async {
    await pumpShell(tester);

    final theme = MixarThemeData.light();
    final hairline = theme.style.borderWidth;
    final expected = BorderSide(color: theme.colors.border, width: hairline);
    BorderSide headerHairline() {
      final box = tester.widget<DecoratedBox>(
        find
            .descendant(
              of: find.byType(AppHeader),
              matching: find.byType(DecoratedBox),
            )
            .first,
      );
      return ((box.decoration as BoxDecoration).border! as Border).bottom;
    }

    final wave = find.byType(WaveformSection);
    final header = tester.getRect(find.byType(AppHeader));
    // The deck row's leading rule overlays the slot edge; the waveform pane
    // still ends where the deck row starts.
    final foot = tester.getRect(
      find.byKey(const ValueKey('mixer-rule-content')),
    );

    // The top hairline belongs to the header so Settings gets it too; the
    // waveform pane starts flush against it.
    expect(header.height, kAppHeaderHeight);
    expect(headerHairline(), expected);
    expect(tester.getTopLeft(wave).dy, header.bottom);

    expect(foot.width, tester.getSize(mixerContentRow).width);
    expect(foot.height, hairline);
    expect(foot.top, tester.getBottomLeft(wave).dy);

    // The reported gap: Settings had no line under the tab bar because the
    // hairline lived on the mixer's waveform pane.
    await tester.tap(find.byIcon(LucideIcons.settings));
    await tester.pumpAndSettle();
    expect(find.byType(SettingsPage), findsOneWidget);
    expect(tester.getTopLeft(find.byType(SettingsPage)).dy, header.bottom);
    expect(headerHairline(), expected);
    expect(tester.takeException(), isNull);
  });

  testWidgets('header shares the deck row surface in both themes', (
    tester,
  ) async {
    for (final theme in [MixarThemeData.dark(), MixarThemeData.light()]) {
      await pumpShell(tester, theme: theme);

      final headerSurface = tester
          .widget<ColoredBox>(
            find
                .descendant(
                  of: find.byType(AppHeader),
                  matching: find.byType(ColoredBox),
                )
                .first,
          )
          .color;
      final deckRowSurface = tester
          .widget<ColoredBox>(
            find
                .ancestor(
                  of: mixerContentRow,
                  matching: find.byType(ColoredBox),
                )
                .first,
          )
          .color;

      // The top bar reads as part of the decks/mixer panel, not the canvas.
      expect(headerSurface, theme.colors.card);
      expect(headerSurface, deckRowSurface);
    }
  });

  testWidgets('stacked decks are split by a hairline across the column', (
    tester,
  ) async {
    await pumpShell(
      tester,
      settings: copyAppSettings(defaultAppSettings(), numDecks: 4),
    );

    final hairline = MixarThemeData.light().style.borderWidth;
    final deckA = tester.getRect(_deck('DECK A'));
    final deckC = tester.getRect(_deck('DECK C'));
    final split = tester.getRect(find.byKey(const ValueKey('deck-split-0')));

    expect(deckC.top - deckA.bottom, hairline);
    expect(split.top, deckA.bottom);
    expect(split.bottom, deckC.top);
    expect(
      split.left,
      tester.getTopLeft(mixerContentRow).dx,
      reason: 'the split runs to the window edge, not the deck card edge',
    );
    expect(
      split.right,
      deckA.right,
      reason: 'the split still stops at the deck edge next to the mixer',
    );

    final rightSplit = tester.getRect(
      find.byKey(const ValueKey('deck-split-1')),
    );
    expect(
      rightSplit.left,
      tester.getRect(_deck('DECK B')).left,
      reason: 'the right column starts the rule at its deck edge',
    );
    expect(
      rightSplit.right,
      tester.getBottomRight(mixerContentRow).dx,
      reason: 'the right column runs the rule to the window edge',
    );
    expect(tester.takeException(), isNull);
  });

  testWidgets('stacked deck split starts on a device pixel at 1.25 dpr', (
    tester,
  ) async {
    const dpr = 1.25;
    await pumpShell(
      tester,
      devicePixelRatio: dpr,
      settings: copyAppSettings(defaultAppSettings(), numDecks: 4),
    );

    // The pair rounds the split against its own origin: the deck row itself
    // starts at a fraction of the body, so only the offset inside the column
    // can be snapped.
    final splitTop =
        tester.getTopLeft(find.byKey(const ValueKey('deck-split-0'))).dy -
        tester.getTopLeft(_deck('DECK A')).dy;
    final devicePixels = splitTop * dpr;

    expect(
      (devicePixels - devicePixels.roundToDouble()).abs(),
      lessThan(1e-6),
      reason: 'the split must start on a whole device pixel',
    );
    expect(tester.takeException(), isNull);
  });

  testWidgets('tempo rail keeps the deck height in drawer layout', (
    tester,
  ) async {
    await pumpShell(
      tester,
      settings: copyAppSettings(
        defaultAppSettings(),
        libraryDrawerEnabled: true,
      ),
    );

    final deck = _deck('DECK A');
    final rail = find.descendant(
      of: deck,
      matching: find.byType(DeckTempoPanel),
    );

    expect(tester.getTopLeft(rail).dy, tester.getTopLeft(deck).dy);
    expect(tester.getBottomLeft(rail).dy, tester.getBottomLeft(deck).dy);
    expect(tester.takeException(), isNull);
  });

  testWidgets('waveform position top stacks the lanes above the deck row', (
    tester,
  ) async {
    await pumpShell(tester);

    final hairline = MixarThemeData.light().style.borderWidth;
    expect(
      tester.getBottomLeft(find.byType(WaveformSection)).dy,
      tester.getTopLeft(mixerContentRow).dy,
      reason: 'the deck row starts where the waveform pane ends',
    );
    expect(find.byType(VerticalWaveformLane), findsNothing);
    expect(find.byType(MixerStrip), findsOneWidget);
    // The pane is above the library, not between decks and library.
    expect(
      tester.getBottomLeft(find.byType(WaveformSection)).dy,
      lessThan(tester.getTopLeft(find.byType(LibraryPanel)).dy - hairline),
    );
  });

  testWidgets('waveform position bottom sits between decks and library', (
    tester,
  ) async {
    await pumpShell(
      tester,
      settings: copyAppSettings(
        defaultAppSettings(),
        waveformPosition: WaveformPositionSetting.bottom,
      ),
    );

    final hairline = MixarThemeData.light().style.borderWidth;
    final wave = find.byType(WaveformSection);
    expect(wave, findsOneWidget);
    expect(
      tester.getTopLeft(wave).dy,
      tester.getBottomLeft(mixerContentRow).dy,
      reason: 'the band follows the deck row; its rule overlays its first row',
    );
    expect(
      tester.getBottomLeft(wave).dy + hairline,
      tester.getTopLeft(find.byType(LibraryPanel)).dy,
      reason: 'the docked library sits under the band',
    );
    expect(find.byType(VerticalWaveformLane), findsNothing);
    expect(find.byType(MixerStrip), findsOneWidget);
    expect(tester.takeException(), isNull);
  });

  testWidgets('waveform position middle horizontal replaces the mixer', (
    tester,
  ) async {
    await pumpShell(
      tester,
      settings: copyAppSettings(
        defaultAppSettings(),
        waveformPosition: WaveformPositionSetting.middleHorizontal,
      ),
    );

    // The lanes take the mixer's column inside the deck row, with the mixer's
    // margins: the band runs edge to edge against the deck cards.
    expect(find.byType(MixerStrip), findsNothing);
    final wave = find.byType(WaveformSection);
    expect(wave, findsOneWidget);
    final lane = tester.getRect(wave);
    final deckRow = tester.getRect(mixerContentRow);
    expect(lane.top, deckRow.top);
    // The mixer is gone, but its foot stays: the crossfader sits under the
    // lanes, spanning the same column.
    final crossfader = tester.getRect(find.byType(CrossfaderPanel));
    expect(lane.bottom, crossfader.top);
    expect(crossfader.left, lane.left);
    expect(crossfader.right, lane.right);
    expect(crossfader.bottom, deckRow.bottom);
    expect(
      tester.getRect(_deck('DECK A')).right,
      lane.left,
      reason: 'the lanes sit flush against the deck columns',
    );
    expect(tester.getRect(_deck('DECK B')).left, lane.right);
    expect(find.byType(VerticalWaveformLane), findsNothing);
    expect(tester.takeException(), isNull);
  });

  testWidgets('waveform position middle vertical replaces the mixer with '
      'one lane per deck', (tester) async {
    await pumpShell(
      tester,
      settings: copyAppSettings(
        defaultAppSettings(),
        waveformPosition: WaveformPositionSetting.middleVertical,
      ),
    );

    // No pane above the decks and no mixer: the lanes are the center column.
    expect(find.byType(WaveformSection), findsNothing);
    expect(find.byType(MixerStrip), findsNothing);
    final lanes = find.byType(VerticalWaveformLane);
    expect(lanes, findsNWidgets(2));

    final deckRow = tester.getRect(mixerContentRow);
    final crossfader = tester.getRect(find.byType(CrossfaderPanel));
    for (final lane in [lanes.at(0), lanes.at(1)]) {
      final rect = tester.getRect(lane);
      expect(rect.top, deckRow.top);
      expect(rect.bottom, crossfader.top);
    }
    expect(crossfader.bottom, deckRow.bottom);
    // Deck A, its lane, Deck B's lane, then Deck B, all flush.
    final deckA = tester.getRect(_deck('DECK A'));
    final deckB = tester.getRect(_deck('DECK B'));
    final laneA = tester.getRect(lanes.at(0));
    final laneB = tester.getRect(lanes.at(1));
    expect(deckA.right, laneA.left);
    expect(laneB.left - laneA.right, MixarThemeData.light().style.borderWidth);
    expect(laneB.right, deckB.left);
    // Each lane names its deck: the rotation would make an in-lane label
    // unreadable, so it is painted outside it.
    expect(
      tester.widget<Text>(find.byKey(const ValueKey('wave-lane-label-0'))).data,
      'A',
    );
    expect(
      tester.widget<Text>(find.byKey(const ValueKey('wave-lane-label-1'))).data,
      'B',
    );
    expect(tester.takeException(), isNull);
  });

  testWidgets('four-deck middle vertical shows a lane per deck in the center', (
    tester,
  ) async {
    await pumpShell(
      tester,
      settings: copyAppSettings(
        defaultAppSettings(),
        numDecks: 4,
        waveformPosition: WaveformPositionSetting.middleVertical,
      ),
    );

    expect(find.byType(MixerStrip), findsNothing);
    final lanes = find.byType(VerticalWaveformLane);
    expect(lanes, findsNWidgets(4));
    // Lanes share the center column, left to right in deck order.
    var previousRight = 0.0;
    for (var deckId = 0; deckId < 4; deckId++) {
      final rect = tester.getRect(lanes.at(deckId));
      expect(rect.left, greaterThan(previousRight));
      previousRight = rect.right;
    }
    expect(tester.takeException(), isNull);
  });

  testWidgets('stacked deck divider runs to the waveform in middle layouts', (
    tester,
  ) async {
    await pumpShell(
      tester,
      settings: copyAppSettings(
        defaultAppSettings(),
        numDecks: 4,
        waveformPosition: WaveformPositionSetting.middleVertical,
      ),
    );

    // The panes drop their inset against the waveform, so the split must too:
    // it spans the deck card, not an inset inside it, and reaches the border.
    final deck = tester.getRect(_deck('DECK A'));
    final split = tester.getRect(find.byKey(const ValueKey('deck-split-0')));
    expect(split.left, tester.getTopLeft(mixerContentRow).dx);
    expect(split.right, deck.right);
    expect(
      split.right,
      tester.getRect(find.byType(VerticalWaveformLane).first).left,
      reason: 'the divider must reach the waveform edge',
    );
    expect(
      tester.getRect(find.byKey(const ValueKey('deck-split-1'))).left,
      tester.getRect(find.byType(VerticalWaveformLane).last).right,
    );
    expect(tester.takeException(), isNull);
  });

  testWidgets('centered crossfader keeps the mixer foot spacing', (
    tester,
  ) async {
    // The center column is flush with the deck cards where the mixer keeps its
    // gutter, so the crossfader re-adds it. Both layouts must place the fader
    // the same distance from the deck card and the pane bottom.
    double? insetFromDeck;
    double? insetFromFoot;
    for (final position in [
      WaveformPositionSetting.top,
      WaveformPositionSetting.middleHorizontal,
    ]) {
      await pumpShell(
        tester,
        settings: copyAppSettings(
          defaultAppSettings(),
          numDecks: 4,
          waveformPosition: position,
        ),
      );

      final deckRow = tester.getRect(mixerContentRow);
      final crossfader = find.byWidgetPredicate(
        (w) => w is FaderSlider && w.orientation == FaderOrientation.horizontal,
      );
      expect(crossfader, findsOneWidget);
      final slider = tester.getRect(crossfader);
      final deck = tester.getRect(_deck('DECK A'));
      insetFromDeck ??= slider.left - deck.right;
      insetFromFoot ??= deckRow.bottom - slider.bottom;
      expect(
        slider.left - deck.right,
        insetFromDeck,
        reason: '$position changed the crossfader gutter',
      );
      expect(
        deckRow.bottom - slider.bottom,
        insetFromFoot,
        reason: '$position changed the crossfader foot spacing',
      );
    }
  });

  testWidgets('hiding the waveform removes the lanes in every position', (
    tester,
  ) async {
    for (final position in WaveformPositionSetting.values) {
      await pumpShell(
        tester,
        settings: copyAppSettings(
          defaultAppSettings(),
          showWaveform: false,
          waveformPosition: position,
        ),
      );

      expect(
        find.byType(WaveformSection),
        findsNothing,
        reason: '$position still renders a waveform section',
      );
      expect(
        find.byType(VerticalWaveformLane),
        findsNothing,
        reason: '$position still renders vertical lanes',
      );
      // The mixer gets its column back, and the deck row starts under the
      // header instead of leaving an empty waveform pane.
      expect(
        find.byType(MixerStrip),
        findsOneWidget,
        reason: '$position lost the mixer with the waveform hidden',
      );
      expect(
        tester.getTopLeft(mixerContentRow).dy,
        tester.getBottomLeft(find.byType(AppHeader)).dy,
        reason: '$position left a gap where the waveform pane used to be',
      );
      expect(tester.takeException(), isNull);
    }
  });
}
