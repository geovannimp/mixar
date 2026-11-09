import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/theme/material_theme.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/core/utils/ui_layout.dart';
import 'package:gui_flutter/providers/library/drawer_providers.dart';
import 'package:gui_flutter/providers/library/history_providers.dart';
import 'package:gui_flutter/providers/library/providers.dart';
import 'package:gui_flutter/providers/settings/settings_providers.dart';
import 'package:gui_flutter/screens/mixer_page.dart';
import 'package:gui_flutter/src/rust/api/library.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';
import 'package:gui_flutter/widgets/mixer/deck_panel.dart';
import 'package:gui_flutter/widgets/mixer/fx_bar.dart';
import 'package:gui_flutter/widgets/mixer/library_panel.dart';
import 'package:gui_flutter/widgets/mixer/mixer_strip.dart';
import 'package:gui_flutter/widgets/mixer/waveform/vertical_lane.dart';
import 'package:gui_flutter/widgets/mixer/waveform_section.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:material_ui/material_ui.dart';
import 'package:panes/panes.dart';

import 'helpers/collection_tracks_override.dart';
import 'support/mixar_material_app.dart';

void main() {
  // The deck row is the mixer's content slot; its key replaces `DeckGrid` as
  // the handle on the row's rectangle.
  final contentRow = find.byKey(const ValueKey('mixer-content-row'));

  const collection = LibraryCollectionSummary(
    id: 'c1',
    name: 'samples',
    kind: 'folder',
    path: '/tmp/samples',
    trackCount: 0,
  );

  Future<void> pumpMixer(
    WidgetTester tester,
    AppSettings settings, {
    Size size = const Size(1200, 800),
  }) async {
    final theme = MixarThemeData.dark();
    tester.view.physicalSize = size;
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);

    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          collectionsProvider.overrideWith((ref) async => [collection]),
          overrideCollectionTracksValue(const []),
          historySessionsProvider.overrideWith((ref) async => const []),
          historyCanResumeProvider.overrideWith((ref) async => false),
          historySettingsBootstrapProvider.overrideWith((ref) {}),
          librarySettingsBootstrapProvider.overrideWith((ref) {}),
          appSettingsProvider.overrideWith((ref) async => settings),
        ],
        child: MaterialApp(
          theme: materialUiThemeFromMixar(theme),
          builder: mixarMaterialAppBuilder(theme),
          home: const Scaffold(body: MixerPage()),
        ),
      ),
    );
    await tester.pump();
    await tester.pumpAndSettle();
  }

  testWidgets('default layout shows waveform, decks, mixer, library, and FX', (
    tester,
  ) async {
    await pumpMixer(tester, defaultAppSettings());
    expect(find.byType(MultiPane), findsWidgets);
    expect(find.byType(WaveformSection), findsOneWidget);
    expect(contentRow, findsOneWidget);
    expect(find.byType(MixerStrip), findsOneWidget);
    expect(find.byType(LibraryPanel), findsOneWidget);
    expect(find.byType(FxBar), findsOneWidget);
    // Two decks at 1200x800 dock the library: the remainder clears its floor.
    expect(tester.getSize(find.byType(LibraryPanel)).height, greaterThan(100));
  });

  testWidgets('hiding the waveform drops the vertical split', (tester) async {
    await pumpMixer(
      tester,
      copyAppSettings(defaultAppSettings(), showWaveform: false),
    );
    expect(find.byType(WaveformSection), findsNothing);
    expect(contentRow, findsOneWidget);
    expect(find.byType(LibraryPanel), findsOneWidget);
    // The deck pane now starts at the top of the page, where the waveform pane
    // used to sit.
    expect(tester.getTopLeft(contentRow).dy, lessThan(1));
  });

  testWidgets('a short window sends the library to the drawer', (tester) async {
    await pumpMixer(tester, defaultAppSettings(), size: const Size(1200, 600));
    expect(find.byType(LibraryPanel), findsNothing);
    final container = ProviderScope.containerOf(
      tester.element(find.byType(MixerPage)),
    );
    expect(container.read(mixerLibraryForcedDrawerProvider), isTrue);
    expect(tester.takeException(), isNull);
  });

  testWidgets('hiding the FX bar keeps decks and library', (tester) async {
    await pumpMixer(
      tester,
      copyAppSettings(defaultAppSettings(), showFxBar: false),
    );
    expect(find.byType(FxBar), findsNothing);
    expect(contentRow, findsOneWidget);
    expect(find.byType(LibraryPanel), findsOneWidget);
  });

  testWidgets('hiding the mixer keeps the decks and drops the channel strip', (
    tester,
  ) async {
    await pumpMixer(
      tester,
      copyAppSettings(defaultAppSettings(), showMixer: false),
    );
    expect(contentRow, findsOneWidget);
    expect(find.byType(MixerStrip), findsNothing);
    expect(find.byType(LibraryPanel), findsOneWidget);
    // The two decks still share the row: deck B keeps its place beside A.
    expect(find.byType(DeckPanel), findsNWidgets(2));
  });

  testWidgets('the drawer mode replaces the docked library, not the panels', (
    tester,
  ) async {
    await pumpMixer(
      tester,
      copyAppSettings(defaultAppSettings(), libraryDrawerEnabled: true),
    );
    expect(contentRow, findsOneWidget);
    // The drawer is a route: the docked panel is gone until it is opened.
    expect(find.byType(LibraryPanel), findsNothing);
    expect(find.byType(MixerStrip), findsOneWidget);
    expect(find.byType(FxBar), findsOneWidget);
  });

  testWidgets('an empty center closes the deck gutter', (tester) async {
    await pumpMixer(
      tester,
      copyAppSettings(
        defaultAppSettings(),
        showWaveform: false,
        showMixer: false,
        showCrossfader: false,
        libraryDrawerEnabled: true,
      ),
    );
    final deckA = tester.getRect(find.byKey(const ValueKey('deck-0')));
    final deckB = tester.getRect(find.byKey(const ValueKey('deck-1')));
    // The decks meet on the seam instead of keeping the mixer's gutter.
    expect(deckB.left - deckA.right, lessThanOrEqualTo(2));
    expect(find.byType(MixerStrip), findsNothing);
  });

  testWidgets('a middle waveform owns the center column, not the mixer', (
    tester,
  ) async {
    await pumpMixer(
      tester,
      copyAppSettings(
        defaultAppSettings(),
        waveformPosition: WaveformPositionSetting.middleVertical,
        showMixer: false,
      ),
    );
    // The lanes take the center column whatever the mixer switch says.
    expect(find.byType(VerticalWaveformLane), findsNWidgets(2));
    expect(find.byType(MixerStrip), findsNothing);
    expect(find.byType(DeckPanel), findsNWidgets(2));
  });

  testWidgets('hiding the crossfader keeps the mixer strip', (tester) async {
    await pumpMixer(
      tester,
      copyAppSettings(defaultAppSettings(), showCrossfader: false),
    );
    expect(find.byType(CrossfaderPanel), findsNothing);
    expect(find.byType(MixerStrip), findsOneWidget);
    expect(find.byType(DeckPanel), findsNWidgets(2));
  });

  testWidgets('the mobile preset centers the waveform in the deck row', (
    tester,
  ) async {
    await pumpMixer(
      tester,
      applyLayoutPreset(defaultAppSettings(), LayoutPreset.mobile),
    );
    final lane = find.descendant(
      of: contentRow,
      matching: find.byType(WaveformSection),
    );
    expect(lane, findsOneWidget);
    expect(find.byType(MixerStrip), findsNothing);
    // The bar stays: hiding it would take the Levels dialog opener with it.
    expect(find.byType(FxBar), findsOneWidget);
    // Centered: the lanes sit between the two decks, not above them.
    expect(tester.getCenter(lane).dx, closeTo(1200 / 2, 40));
    final deckA = tester.getRect(find.byKey(const ValueKey('deck-0')));
    final deckB = tester.getRect(find.byKey(const ValueKey('deck-1')));
    // The lanes sit edge to edge against the deck cards: the mixer's gutters
    // close when the center column is not the mixer.
    expect(tester.getRect(lane).left, greaterThanOrEqualTo(deckA.right));
    expect(tester.getRect(lane).right, lessThanOrEqualTo(deckB.left));
  });

  testWidgets('the crossfader also leaves the middle waveform column', (
    tester,
  ) async {
    await pumpMixer(
      tester,
      copyAppSettings(
        defaultAppSettings(),
        waveformPosition: WaveformPositionSetting.middleHorizontal,
        showCrossfader: false,
      ),
    );
    expect(find.byType(CrossfaderPanel), findsNothing);
    expect(find.byType(WaveformSection), findsOneWidget);
    expect(find.byType(MixerStrip), findsNothing);
  });
}
