import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/core/utils/ui_layout.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';

void main() {
  test('desktop keeps every panel docked', () {
    final layout = applyLayoutPreset(
      defaultAppSettings(),
      LayoutPreset.desktop,
    );
    expect(layout.showWaveform, isTrue);
    expect(layout.showMixer, isTrue);
    expect(layout.showFxBar, isTrue);
    expect(layout.libraryDrawerEnabled, isFalse);
  });

  test('tablet moves the library into the drawer', () {
    final layout = applyLayoutPreset(defaultAppSettings(), LayoutPreset.tablet);
    expect(layout.showWaveform, isTrue);
    expect(layout.showMixer, isTrue);
    expect(layout.showFxBar, isTrue);
    expect(layout.libraryDrawerEnabled, isTrue);
    expect(layout.libraryDrawerSide, LibraryDrawerSideSetting.bottom);
    // Re-picking Mobile keeps a centered waveform the user already chose.
    expect(
      applyLayoutPreset(
        copyAppSettings(
          defaultAppSettings(),
          waveformPosition: WaveformPositionSetting.middleVertical,
        ),
        LayoutPreset.mobile,
      ).waveformPosition,
      WaveformPositionSetting.middleVertical,
    );
  });

  test('mobile centers the waveform between the decks', () {
    final layout = applyLayoutPreset(defaultAppSettings(), LayoutPreset.mobile);
    expect(layout.showWaveform, isTrue);
    expect(layout.waveformPosition, WaveformPositionSetting.middleHorizontal);
    expect(layout.showMixer, isFalse);
    // The FX bar stays in every preset: it carries the Levels dialog opener.
    expect(layout.showFxBar, isTrue);
    expect(layout.libraryDrawerEnabled, isTrue);
    expect(layout.libraryDrawerSide, LibraryDrawerSideSetting.bottom);
  });

  test('presets leave audio and deck count alone', () {
    final base = copyAppSettings(
      defaultAppSettings(),
      numDecks: 4,
      showCrossfader: false,
      waveformPosition: WaveformPositionSetting.middleVertical,
    );
    for (final preset in kLayoutPresets) {
      final layout = applyLayoutPreset(base, preset);
      expect(layout.numDecks, 4);
      expect(layout.targetLufs, base.targetLufs);
      expect(layout.waveformDisplayMode, base.waveformDisplayMode);
      // The crossfader is not part of any preset: it stays as the user left it.
      expect(layout.showCrossfader, isFalse, reason: preset.name);
    }
  });

  test(
    'desktop and tablet keep top and bottom, and move a centered one up',
    () {
      for (final preset in [LayoutPreset.desktop, LayoutPreset.tablet]) {
        for (final position in [
          WaveformPositionSetting.top,
          WaveformPositionSetting.bottom,
        ]) {
          expect(
            applyLayoutPreset(
              copyAppSettings(defaultAppSettings(), waveformPosition: position),
              preset,
            ).waveformPosition,
            position,
            reason: '${preset.name} moved $position',
          );
        }
        for (final position in [
          WaveformPositionSetting.middleHorizontal,
          WaveformPositionSetting.middleVertical,
        ]) {
          // The lanes own the column the strip needs, so they go back on top.
          expect(
            applyLayoutPreset(
              copyAppSettings(defaultAppSettings(), waveformPosition: position),
              preset,
            ).waveformPosition,
            WaveformPositionSetting.top,
            reason: '${preset.name} kept $position',
          );
        }
      }
    },
  );

  test('custom changes nothing', () {
    final base = copyAppSettings(defaultAppSettings(), showFxBar: false);
    expect(applyLayoutPreset(base, LayoutPreset.custom), base);
  });

  test('each preset round-trips through layoutPresetFor', () {
    for (final preset in kLayoutPresets) {
      final layout = applyLayoutPreset(defaultAppSettings(), preset);
      expect(layoutPresetFor(layout), preset);
    }
  });

  test('a toggle that breaks the bundle reads as custom', () {
    final layout = copyAppSettings(
      applyLayoutPreset(defaultAppSettings(), LayoutPreset.tablet),
      showMixer: false,
    );
    expect(layoutPresetFor(layout), LayoutPreset.custom);
  });

  test('preset identity follows the waveform position', () {
    final mobile = applyLayoutPreset(defaultAppSettings(), LayoutPreset.mobile);
    final tablet = applyLayoutPreset(defaultAppSettings(), LayoutPreset.tablet);
    final expected = {
      WaveformPositionSetting.middleHorizontal: (mobile, LayoutPreset.mobile),
      WaveformPositionSetting.middleVertical: (mobile, LayoutPreset.mobile),
      WaveformPositionSetting.top: (mobile, LayoutPreset.custom),
      WaveformPositionSetting.bottom: (mobile, LayoutPreset.custom),
    };
    for (final MapEntry(key: position, value: (base, preset))
        in expected.entries) {
      expect(
        layoutPresetFor(copyAppSettings(base, waveformPosition: position)),
        preset,
        reason: 'mobile with ${position.name}',
      );
    }

    final tabletExpected = {
      WaveformPositionSetting.middleHorizontal: LayoutPreset.custom,
      WaveformPositionSetting.middleVertical: LayoutPreset.custom,
      WaveformPositionSetting.top: LayoutPreset.tablet,
      WaveformPositionSetting.bottom: LayoutPreset.tablet,
    };
    for (final MapEntry(key: position, value: preset)
        in tabletExpected.entries) {
      expect(
        layoutPresetFor(copyAppSettings(tablet, waveformPosition: position)),
        preset,
        reason: 'tablet with ${position.name}',
      );
    }
  });

  test('drawer side and waveform position are not preset identity', () {
    final layout = copyAppSettings(
      applyLayoutPreset(defaultAppSettings(), LayoutPreset.tablet),
      libraryDrawerSide: LibraryDrawerSideSetting.right,
      waveformPosition: WaveformPositionSetting.bottom,
    );
    expect(layoutPresetFor(layout), LayoutPreset.tablet);
  });
}
