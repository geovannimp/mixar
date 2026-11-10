import 'dart:typed_data';

import 'package:flutter/rendering.dart';
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
import 'package:gui_flutter/widgets/settings/settings_visual_choice.dart';
import 'package:gui_flutter/widgets/shell/mixar_select.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:material_ui/material_ui.dart';
import 'package:shadcn_ui/shadcn_ui.dart';

import 'support/mixar_material_app.dart';

void main() {
  Future<void> pumpPanel(
    WidgetTester tester, {
    AppSettings? draft,
    ValueChanged<AppSettings>? onChanged,
    Key? captureKey,
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
              child: RepaintBoundary(
                key: captureKey,
                child: SettingsLayoutPanel(
                  draft: settings,
                  onChanged: onChanged ?? (_) {},
                ),
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

  Finder waveformChoice() =>
      find.byType(SettingsVisualChoice<WaveformPositionSetting>);
  Finder drawerChoice() =>
      find.byType(SettingsVisualChoice<LibraryDrawerSideSetting>);

  SettingsSelect<LayoutPreset> presetSelect(WidgetTester tester) =>
      tester.widget<SettingsSelect<LayoutPreset>>(
        find.byType(SettingsSelect<LayoutPreset>),
      );

  Finder choiceOption(Finder choice, String label) =>
      find.descendant(of: choice, matching: find.text(label));

  /// Header switch of the card titled [title].
  Finder panelSwitch(String title) => find.descendant(
    of: find.byWidgetPredicate(
      (widget) => widget is SettingsSectionCard && widget.title == title,
    ),
    matching: find.byType(SettingsHeaderSwitch),
  );

  testWidgets('layout panel exposes the waveform position', (tester) async {
    await pumpPanel(tester);
    expect(find.text('Waveform position'), findsOneWidget);
    expect(find.text('POSITION'), findsOneWidget);

    for (final label in [
      'Top',
      'Middle horizontal',
      'Middle vertical',
      'Bottom',
    ]) {
      expect(
        choiceOption(waveformChoice(), label),
        findsOneWidget,
        reason: 'missing $label option',
      );
    }
  }, semanticsEnabled: false);

  testWidgets('the default layout reads as the Desktop preset', (tester) async {
    await pumpPanel(tester);
    expect(find.text('Presets'), findsOneWidget);
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

  testWidgets('choosing middle vertical emits the position', (tester) async {
    AppSettings? changed;
    await pumpPanel(tester, onChanged: (next) => changed = next);

    await tapAndSettle(
      tester,
      choiceOption(waveformChoice(), 'Middle vertical'),
    );

    expect(changed?.waveformPosition, WaveformPositionSetting.middleVertical);
  }, semanticsEnabled: false);

  testWidgets('hiding the waveform drops the position choice', (tester) async {
    AppSettings? changed;
    await pumpPanel(
      tester,
      draft: copyAppSettings(defaultAppSettings(), showWaveform: false),
      onChanged: (next) => changed = next,
    );

    expect(
      find.byType(SettingsVisualChoice<WaveformPositionSetting>),
      findsNothing,
    );
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

  testWidgets('layout panel hosts panels, waveform, and drawer', (
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
    expect(find.text('Select style'), findsOneWidget);
    expect(find.byType(SettingsSelect<SelectStyleSetting>), findsOneWidget);
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

  testWidgets('the crossfader switch survives the strip', (tester) async {
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

    await tester.ensureVisible(crossfader);
    await tester.pumpAndSettle();
    await tester.tap(crossfader);
    await tester.pump();

    expect(changed?.showCrossfader, isFalse);
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
  }, semanticsEnabled: false);

  testWidgets('toggling a panel off emits just that field', (tester) async {
    AppSettings? changed;
    await pumpPanel(tester, onChanged: (next) => changed = next);

    await tapAndSettle(tester, panelSwitch('FX bar'));

    expect(changed?.showFxBar, isFalse);
    expect(changed?.showMixer, isTrue);
    expect(changed?.showWaveform, isTrue);
  }, semanticsEnabled: false);

  testWidgets('the drawer card hides its position until the drawer is on', (
    tester,
  ) async {
    await pumpPanel(tester);
    // Only the waveform card shows a position label while the drawer is off.
    expect(find.text('POSITION'), findsOneWidget);
    expect(drawerChoice(), findsNothing);
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
    expect(
      tester
          .widget<SettingsVisualChoice<LibraryDrawerSideSetting>>(
            drawerChoice(),
          )
          .axis,
      Axis.horizontal,
    );
    expect(choiceOption(drawerChoice(), 'Bottom'), findsOneWidget);
  }, semanticsEnabled: false);

  testWidgets('choosing a drawer edge emits it', (tester) async {
    AppSettings? changed;
    await pumpPanel(
      tester,
      draft: copyAppSettings(defaultAppSettings(), libraryDrawerEnabled: true),
      onChanged: (next) => changed = next,
    );

    await tapAndSettle(tester, choiceOption(drawerChoice(), 'Left'));

    expect(changed?.libraryDrawerSide, LibraryDrawerSideSetting.left);
  }, semanticsEnabled: false);

  testWidgets('select style keeps the bottom stroke of its focus ring', (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1200, 2400);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);
    final captureKey = GlobalKey();
    await pumpPanel(tester, captureKey: captureKey);

    // The select style control ends its card body, so the ring's bottom stroke
    // must survive the card body's clip.
    final select = find.byType(MixarSelect<SelectStyleSetting>);
    expect(select, findsOneWidget);
    await _focusSelect(tester, select);

    final rect = tester.getRect(select);
    final capture = await _capturePanel(tester, captureKey);
    final primary = MixarThemeData.dark().colors.primary;
    final column = (rect.center.dx - capture.origin.dx).round();
    final bottomEdge = (rect.bottom - capture.origin.dy).round();
    final strokeRows = [
      for (final offset in const [1, 2, 3, 4])
        if (_hasColor(
          capture.rgba,
          capture.stride,
          column,
          bottomEdge + offset,
          primary,
        ))
          offset,
    ];
    expect(strokeRows, isNotEmpty, reason: 'the bottom ring stroke is clipped');
  }, semanticsEnabled: false);
}

/// Focuses the shad control inside [select] and lets the focus ring settle.
Future<void> _focusSelect(WidgetTester tester, Finder select) async {
  tester
      .widget<ShadFocusable>(
        find.descendant(of: select, matching: find.byType(ShadFocusable)),
      )
      .focusNode
      ?.requestFocus();
  await tester.pumpAndSettle();
}

/// RGBA pixels of the panel captured under [captureKey], with its global
/// origin so test rects can be turned into raster coordinates.
Future<({Uint8List rgba, int stride, Offset origin})> _capturePanel(
  WidgetTester tester,
  GlobalKey captureKey,
) async {
  final boundary =
      captureKey.currentContext!.findRenderObject()! as RenderRepaintBoundary;
  final origin = boundary.localToGlobal(Offset.zero);
  // Rasterizing needs the real event loop; toImage deadlocks inside the
  // fake-async zone tester.runAsync would otherwise keep.
  final raster = await tester.runAsync(() async {
    final image = await boundary.toImage();
    try {
      final data = await image.toByteData();
      return (rgba: data!.buffer.asUint8List(), width: image.width);
    } finally {
      image.dispose();
    }
  });
  return (rgba: raster!.rgba, stride: raster.width * 4, origin: origin);
}

/// Whether the pixel at ([x], [y]) matches [color].
bool _hasColor(Uint8List rgba, int stride, int x, int y, Color color) {
  final argb = color.toARGB32();
  final i = y * stride + x * 4;
  return rgba[i] == ((argb >> 16) & 0xFF) &&
      rgba[i + 1] == ((argb >> 8) & 0xFF) &&
      rgba[i + 2] == (argb & 0xFF);
}
