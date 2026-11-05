import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';

/// Named starting points for which mixer panels are visible (Settings →
/// Layout).
///
/// A preset is only a bundle of view-only toggles: it never touches the deck
/// count or any audio setting, so applying one saves without restarting the
/// engine. It does pick the waveform position and the drawer side: Mobile
/// centers the lanes, Desktop and Tablet move a centered waveform back on top,
/// and a preset that turns the drawer on opens it from the bottom. [custom] is
/// never stored — it is what [layoutPresetFor] reports once the individual
/// toggles no longer line up with a named preset.
enum LayoutPreset {
  desktop,
  tablet,
  mobile,
  custom;

  /// Whether [position] still reads as this preset's waveform placement.
  ///
  /// Mobile is the phone layout, so it only matches a centered waveform.
  /// Desktop and Tablet promise the mixer strip, which the centered lanes
  /// replace, so they only match a top or bottom waveform. Between themselves
  /// they leave the exact position to the select.
  bool acceptsWaveformPosition(WaveformPositionSetting position) =>
      switch (this) {
        LayoutPreset.mobile => _isCentered(position),
        LayoutPreset.desktop || LayoutPreset.tablet => !_isCentered(position),
        LayoutPreset.custom => true,
      };
}

/// Whether [position] hands the mixer's center column to the waveform.
bool _isCentered(WaveformPositionSetting position) =>
    position == WaveformPositionSetting.middleHorizontal ||
    position == WaveformPositionSetting.middleVertical;

/// [position], moved on top when it would take the mixer's column.
///
/// Desktop and Tablet keep top and bottom as the user set them; only the
/// centered positions have to move, because they hide the strip the preset
/// promises.
WaveformPositionSetting _centeredPosition(WaveformPositionSetting position) =>
    _isCentered(position) ? WaveformPositionSetting.top : position;

/// Named presets, in the order the Settings select lists them.
const List<LayoutPreset> kLayoutPresets = [
  LayoutPreset.desktop,
  LayoutPreset.tablet,
  LayoutPreset.mobile,
];

/// The panels Desktop and Tablet share: waveform, mixer strip, and FX bar on,
/// with the library docked or drawn.
///
/// Desktop and Tablet promise the mixer strip, and a centered waveform owns
/// that column: leaving one centered would show a preset label with no mixer
/// under it, so the lanes go back on top. A drawer preset opens it from the
/// bottom, which is the device arrangement it describes.
AppSettings _mixerPanels(AppSettings base, {required bool drawer}) =>
    copyAppSettings(
      base,
      showWaveform: true,
      waveformPosition: _centeredPosition(base.waveformPosition),
      showMixer: true,
      showFxBar: true,
      libraryDrawerEnabled: drawer,
      libraryDrawerSide: drawer
          ? LibraryDrawerSideSetting.bottom
          : base.libraryDrawerSide,
    );

/// Apply [preset]'s panels to [base]. [LayoutPreset.custom] returns [base].
///
/// Desktop keeps every panel docked, tablet moves the library into the drawer
/// to give the deck row the full width, and mobile drops the mixer strip and
/// centers the waveform between the decks, so the decks, the lanes, and the
/// drawer library fit a phone. Mobile is the one preset that centers the
/// waveform; desktop and tablet keep a top or bottom waveform where it is and
/// only lift a centered one back on top, because they promise the strip. The
/// FX bar stays on everywhere: it is the only entry to master and headphone
/// levels. The library itself is never hidden: only docked or drawn.
AppSettings applyLayoutPreset(AppSettings base, LayoutPreset preset) =>
    switch (preset) {
      LayoutPreset.desktop => _mixerPanels(base, drawer: false),
      LayoutPreset.tablet => _mixerPanels(base, drawer: true),
      LayoutPreset.mobile => copyAppSettings(
        base,
        showWaveform: true,
        // Either middle position is the phone arrangement: keep the one in use
        // and center only a top or bottom waveform.
        waveformPosition: _isCentered(base.waveformPosition)
            ? base.waveformPosition
            : WaveformPositionSetting.middleHorizontal,
        showMixer: false,
        showFxBar: true,
        libraryDrawerEnabled: true,
        libraryDrawerSide: LibraryDrawerSideSetting.bottom,
      ),
      LayoutPreset.custom => base,
    };

/// Which named preset [settings] currently matches, else [LayoutPreset.custom].
///
/// Derived rather than stored, so the select can never show a preset the
/// visible panels contradict (a hand-edited settings.json, a toggle flipped
/// after picking a preset). The drawer side is not part of a preset's identity
/// — only whether the library is docked or drawn — and the waveform position
/// only counts for mobile, which is the one preset that places the lanes.
LayoutPreset layoutPresetFor(AppSettings settings) {
  for (final preset in kLayoutPresets) {
    final probe = applyLayoutPreset(settings, preset);
    if (probe.showWaveform == settings.showWaveform &&
        probe.showMixer == settings.showMixer &&
        probe.showFxBar == settings.showFxBar &&
        probe.libraryDrawerEnabled == settings.libraryDrawerEnabled &&
        preset.acceptsWaveformPosition(settings.waveformPosition)) {
      return preset;
    }
  }
  return LayoutPreset.custom;
}
