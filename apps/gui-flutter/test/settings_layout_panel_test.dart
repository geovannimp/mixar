import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/core/utils/ui_layout.dart';
import 'package:gui_flutter/providers/settings/settings_providers.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';
import 'package:gui_flutter/widgets/settings/settings_header_switch.dart';
import 'package:gui_flutter/widgets/settings/settings_layout_panel.dart';
import 'package:gui_flutter/widgets/settings/settings_section_card.dart';
import 'package:gui_flutter/widgets/settings/settings_select.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:material_ui/material_ui.dart';
import 'package:shadcn_ui/shadcn_ui.dart';

import 'support/mixar_material_app.dart';

void main() {
  Future<void> pumpPanel(
    WidgetTester tester, {
    AppSettings? draft,
    ValueChanged<AppSettings>? onChanged,
  }) async {
    final settings = draft ?? defaultAppSettings();
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          // Desktop selects keep the dropdown in-tree, so the test can pick an
          // option without mounting a dialog.
          appSettingsProvider.overrideWith(
            (ref) async => copyAppSettings(
              settings,
              selectStyle: SelectStyleSetting.desktop,
            ),
          ),
        ],
        child: mixarTestMaterialApp(
          theme: MixarThemeData.dark(),
          home: Scaffold(
            body: SingleChildScrollView(
              child: SettingsLayoutPanel(
                draft: settings,
                onChanged: onChanged ?? (_) {},
              ),
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();
  }

  /// Scrolls [finder] into view, taps it, and lets the panel settle.
  Future<void> tapAndSettle(WidgetTester tester, Finder finder) async {
    await tester.ensureVisible(finder);
    await tester.pumpAndSettle();
    await tester.tap(finder);
    await tester.pumpAndSettle();
  }

  SettingsSelect<LayoutPreset> presetSelect(WidgetTester tester) =>
      tester.widget<SettingsSelect<LayoutPreset>>(
        find.byType(SettingsSelect<LayoutPreset>),
      );

  /// Header switch of the card titled [title].
  Finder panelSwitch(String title) => find.descendant(
    of: find.byWidgetPredicate(
      (widget) => widget is SettingsSectionCard && widget.title == title,
    ),
    matching: find.byType(SettingsHeaderSwitch),
  );

  Future<void> openSelect(WidgetTester tester) async {
    final select = find.byType(ShadSelect<WaveformPositionSetting>);
    // The waveform card sits below the preset and panel cards.
    await tapAndSettle(tester, select);
  }

  testWidgets('layout panel exposes the waveform position', (tester) async {
    await pumpPanel(tester);
    expect(find.text('Waveform position'), findsOneWidget);
    expect(find.text('POSITION'), findsOneWidget);
    expect(find.text('Top'), findsOneWidget);

    await openSelect(tester);
    for (final label in [
      'Top',
      'Middle horizontal',
      'Middle vertical',
      'Bottom',
    ]) {
      expect(find.text(label), findsWidgets, reason: 'missing $label option');
    }
  }, semanticsEnabled: false);

  testWidgets('choosing middle vertical emits the position', (tester) async {
    AppSettings? changed;
    await pumpPanel(tester, onChanged: (next) => changed = next);

    await openSelect(tester);
    await tester.tap(find.text('Middle vertical'));
    await tester.pumpAndSettle();

    expect(changed?.waveformPosition, WaveformPositionSetting.middleVertical);
  }, semanticsEnabled: false);

  testWidgets('hiding the waveform drops the position select', (tester) async {
    AppSettings? changed;
    await pumpPanel(
      tester,
      draft: copyAppSettings(defaultAppSettings(), showWaveform: false),
      onChanged: (next) => changed = next,
    );

    expect(find.byType(ShadSelect<WaveformPositionSetting>), findsNothing);
    expect(find.text('POSITION'), findsNothing);

    await tapAndSettle(tester, panelSwitch('Waveform position'));
    expect(changed?.showWaveform, isTrue);
  }, semanticsEnabled: false);

  testWidgets('the switch starts on and enables hiding', (tester) async {
    AppSettings? changed;
    await pumpPanel(tester, onChanged: (next) => changed = next);

    final toggleFinder = panelSwitch('Waveform position');
    final toggle = tester.widget<SettingsHeaderSwitch>(toggleFinder);
    expect(toggle.value, isTrue);

    await tapAndSettle(tester, toggleFinder);
    expect(changed?.showWaveform, isFalse);
  }, semanticsEnabled: false);

  testWidgets('layout panel hosts presets, panels, waveform, and drawer', (
    tester,
  ) async {
    await pumpPanel(tester);
    expect(find.text('Layout'), findsOneWidget);
    expect(find.text('Presets'), findsOneWidget);
    expect(find.text('Deck layout'), findsOneWidget);
    expect(find.text('Mixer strip'), findsOneWidget);
    expect(find.text('FX bar'), findsOneWidget);
    expect(find.text('Waveform position'), findsOneWidget);
    expect(find.text('Library drawer'), findsOneWidget);
    // The crossfader lives inside the mixer strip card.
    expect(find.text('Crossfader'), findsOneWidget);
    expect(panelSwitch('Mixer strip'), findsOneWidget);
    expect(panelSwitch('FX bar'), findsOneWidget);
  }, semanticsEnabled: false);

  testWidgets('the deck layout card emits four decks with sampler defaults', (
    tester,
  ) async {
    AppSettings? changed;
    await pumpPanel(tester, onChanged: (next) => changed = next);

    final decks = tester.widget<SettingsSelect<int>>(
      find.byType(SettingsSelect<int>),
    );
    expect(decks.value, 2);
    decks.onChanged(4);
    await tester.pump();

    expect(changed?.numDecks, 4);
    expect(changed?.deckDefaultSamplerBankId, [null, null, null, null]);
  }, semanticsEnabled: false);

  testWidgets('the crossfader switch hides only the crossfader', (
    tester,
  ) async {
    AppSettings? changed;
    await pumpPanel(tester, onChanged: (next) => changed = next);

    final crossfader = find.text('Crossfader');
    await tester.ensureVisible(crossfader);
    await tester.tap(crossfader);
    await tester.pump();

    expect(changed?.showCrossfader, isFalse);
    expect(changed?.showMixer, isTrue);
    expect(changed?.showFxBar, isTrue);
  }, semanticsEnabled: false);

  testWidgets('the mixer strip header switch hides only the strip', (
    tester,
  ) async {
    AppSettings? changed;
    await pumpPanel(tester, onChanged: (next) => changed = next);

    await tapAndSettle(tester, panelSwitch('Mixer strip'));

    expect(changed?.showMixer, isFalse);
    expect(changed?.showFxBar, isTrue);
  }, semanticsEnabled: false);

  testWidgets('hiding the mixer strip drops the crossfader switch', (
    tester,
  ) async {
    await pumpPanel(
      tester,
      draft: copyAppSettings(defaultAppSettings(), showMixer: false),
    );
    expect(find.text('Mixer strip'), findsOneWidget);
    expect(find.text('Crossfader'), findsNothing);
  }, semanticsEnabled: false);

  testWidgets('the meters select offers none, mono, and stereo', (
    tester,
  ) async {
    AppSettings? changed;
    await pumpPanel(tester, onChanged: (next) => changed = next);

    expect(find.text('MIXER METERS'), findsOneWidget);
    final meters = tester.widget<SettingsSelect<MixerMetersSetting>>(
      find.byType(SettingsSelect<MixerMetersSetting>),
    );
    expect(meters.value, MixerMetersSetting.mono);
    expect(meters.options, [
      MixerMetersSetting.none,
      MixerMetersSetting.mono,
      MixerMetersSetting.stereo,
    ]);

    meters.onChanged(MixerMetersSetting.none);
    await tester.pump();
    expect(changed?.mixerMeters, MixerMetersSetting.none);

    meters.onChanged(MixerMetersSetting.stereo);
    await tester.pump();
    expect(changed?.mixerMeters, MixerMetersSetting.stereo);

    // The popover offers the three modes, None included.
    await tapAndSettle(tester, find.byType(ShadSelect<MixerMetersSetting>));
    for (final label in ['None', 'Mono', 'Stereo']) {
      expect(find.text(label), findsWidgets, reason: 'missing $label option');
    }
  }, semanticsEnabled: false);

  testWidgets('the default layout reads as the Desktop preset', (tester) async {
    await pumpPanel(tester);
    expect(presetSelect(tester).value, LayoutPreset.desktop);
  }, semanticsEnabled: false);

  testWidgets('picking Tablet bundles the tablet panels', (tester) async {
    AppSettings? changed;
    await pumpPanel(tester, onChanged: (next) => changed = next);

    presetSelect(tester).onChanged(LayoutPreset.tablet);
    await tester.pump();

    expect(changed?.showWaveform, isTrue);
    expect(changed?.showMixer, isTrue);
    expect(changed?.showFxBar, isTrue);
    expect(changed?.libraryDrawerEnabled, isTrue);
    expect(changed?.libraryDrawerSide, LibraryDrawerSideSetting.bottom);
  }, semanticsEnabled: false);

  testWidgets('picking Mobile centers the waveform and drops the mixer', (
    tester,
  ) async {
    AppSettings? changed;
    await pumpPanel(tester, onChanged: (next) => changed = next);

    presetSelect(tester).onChanged(LayoutPreset.mobile);
    await tester.pump();

    expect(changed?.showWaveform, isTrue);
    expect(changed?.waveformPosition, WaveformPositionSetting.middleHorizontal);
    expect(changed?.showMixer, isFalse);
    // The FX bar stays: it is the only entry to the Levels dialog.
    expect(changed?.showFxBar, isTrue);
    expect(changed?.libraryDrawerEnabled, isTrue);
  }, semanticsEnabled: false);

  testWidgets(
    'the crossfader switch survives the strip and the meters do not',
    (tester) async {
      AppSettings? changed;
      await pumpPanel(
        tester,
        draft: copyAppSettings(
          defaultAppSettings(),
          showMixer: false,
          waveformPosition: WaveformPositionSetting.middleHorizontal,
        ),
        onChanged: (next) => changed = next,
      );

      // A middle waveform keeps the center column and its foot.
      final crossfader = find.text('Crossfader');
      expect(crossfader, findsOneWidget);
      expect(find.text('MIXER METERS'), findsNothing);

      await tester.ensureVisible(crossfader);
      await tester.pumpAndSettle();
      await tester.tap(crossfader);
      await tester.pump();

      expect(changed?.showCrossfader, isFalse);
    },
    semanticsEnabled: false,
  );

  testWidgets('a middle waveform hides the meters with the strip', (
    tester,
  ) async {
    // The strip switch stays on, but the lanes hold the column, so the meter
    // select would be inert: it goes with the strip's channels.
    await pumpPanel(
      tester,
      draft: copyAppSettings(
        defaultAppSettings(),
        waveformPosition: WaveformPositionSetting.middleVertical,
      ),
    );
    expect(find.text('Crossfader'), findsOneWidget);
    expect(find.text('MIXER METERS'), findsNothing);
  }, semanticsEnabled: false);

  testWidgets('without any center column the crossfader card is empty', (
    tester,
  ) async {
    await pumpPanel(
      tester,
      draft: copyAppSettings(
        defaultAppSettings(),
        showMixer: false,
        showWaveform: false,
      ),
    );
    expect(find.text('Crossfader'), findsNothing);
    expect(find.text('MIXER METERS'), findsNothing);
  }, semanticsEnabled: false);

  testWidgets('toggling a panel off emits just that field', (tester) async {
    AppSettings? changed;
    await pumpPanel(tester, onChanged: (next) => changed = next);

    await tapAndSettle(tester, panelSwitch('FX bar'));

    expect(changed?.showFxBar, isFalse);
    expect(changed?.showMixer, isTrue);
    expect(changed?.showWaveform, isTrue);
  }, semanticsEnabled: false);

  testWidgets('toggles that match no preset read as Custom', (tester) async {
    await pumpPanel(
      tester,
      draft: copyAppSettings(defaultAppSettings(), showFxBar: false),
    );
    expect(presetSelect(tester).value, LayoutPreset.custom);
    expect(find.text('Custom'), findsOneWidget);
    expect(
      find.text('No preset matches the visible panels below.'),
      findsOneWidget,
    );
  }, semanticsEnabled: false);

  testWidgets('the drawer card hides its position until the drawer is on', (
    tester,
  ) async {
    await pumpPanel(tester);
    // Only the waveform card shows a position label while the drawer is off.
    expect(find.text('POSITION'), findsOneWidget);
  }, semanticsEnabled: false);

  testWidgets('the drawer position shows when the drawer is enabled', (
    tester,
  ) async {
    await pumpPanel(
      tester,
      draft: copyAppSettings(defaultAppSettings(), libraryDrawerEnabled: true),
    );
    // Waveform position and drawer position, both on screen.
    expect(find.text('POSITION'), findsNWidgets(2));
    expect(find.text('Bottom'), findsOneWidget);
  }, semanticsEnabled: false);
}
