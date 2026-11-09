import 'package:flutter/foundation.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';
import 'package:gui_flutter/widgets/mixer/layout/mixer_slot.dart';
import 'package:gui_flutter/widgets/mixer/mixer_strip.dart';

/// Where the waveform goes, in the terms the panes think in.
enum MixerLayoutKind {
  waveformTop,
  waveformBottom,
  centreHorizontal,
  centreVertical,
}

/// Everything the solver and the plan need from settings, as plain values.
///
/// The page solves the layout from this alone, so a settings change is one
/// comparison instead of a rebuild loop, and the solver stays testable without
/// a Riverpod container.
@immutable
class MixerLayoutConfig {
  const new({
    required this.kind,
    required this.numDecks,
    required this.showWaveform,
    required this.showMixer,
    required this.showCrossfader,
    required this.showFxBar,
    required this.preferDockedLibrary,
    required this.mixerColumnWidth,
    required this.laneColumnWidth,
  });

  // A named constructor would trip `unnecessary_type_name_in_constructor`,
  // which this app promotes to a warning, so the settings edge is a static
  // factory function instead.
  // ignore: prefer_constructors_over_static_methods
  static MixerLayoutConfig fromSettings(AppSettings settings) {
    final fourDecks = settings.numDecks == 4;
    final mixerWidth = MixerStrip.widthFor(
      fourDecks: fourDecks,
      meters: showsDeckMeters(settings),
    );
    return MixerLayoutConfig(
      kind: switch (settings.waveformPosition) {
        WaveformPositionSetting.top => MixerLayoutKind.waveformTop,
        WaveformPositionSetting.bottom => MixerLayoutKind.waveformBottom,
        WaveformPositionSetting.middleHorizontal =>
          MixerLayoutKind.centreHorizontal,
        WaveformPositionSetting.middleVertical =>
          MixerLayoutKind.centreVertical,
      },
      numDecks: settings.numDecks,
      showWaveform: settings.showWaveform,
      showMixer: settings.showMixer,
      showCrossfader: settings.showCrossfader,
      showFxBar: settings.showFxBar,
      preferDockedLibrary: !settings.libraryDrawerEnabled,
      // The mixer strip owns the deck gutter on each side; the centre pane is
      // pinned to its full outer width.
      mixerColumnWidth: mixerWidth + 2 * MixerPaneMetrics.mixerBleed,
      laneColumnWidth: fourDecks
          ? MixerPaneMetrics.laneWidthFour
          : MixerPaneMetrics.laneWidthTwo,
    );
  }

  /// Which waveform placement the deck row is asked for.
  final MixerLayoutKind kind;

  /// Active decks: 2 or 4.
  final int numDecks;

  /// Whether the deck waveforms are drawn at all.
  final bool showWaveform;

  /// Whether the mixer strip may take the centre column.
  final bool showMixer;

  /// Whether the crossfader sits under a waveform centre column.
  final bool showCrossfader;

  /// Whether the FX bar rides at the foot of the page.
  final bool showFxBar;

  /// Whether a docked library is preferred; false keeps it in the drawer.
  final bool preferDockedLibrary;

  /// Centre column width when the mixer takes it, gutters included.
  final double mixerColumnWidth;

  /// Centre column width when the waveform takes it.
  final double laneColumnWidth;

  bool get fourDecks => numDecks == 4;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is MixerLayoutConfig &&
          other.kind == kind &&
          other.numDecks == numDecks &&
          other.showWaveform == showWaveform &&
          other.showMixer == showMixer &&
          other.showCrossfader == showCrossfader &&
          other.showFxBar == showFxBar &&
          other.preferDockedLibrary == preferDockedLibrary &&
          other.mixerColumnWidth == mixerColumnWidth &&
          other.laneColumnWidth == laneColumnWidth;

  @override
  int get hashCode => Object.hash(
    kind,
    numDecks,
    showWaveform,
    showMixer,
    showCrossfader,
    showFxBar,
    preferDockedLibrary,
    mixerColumnWidth,
    laneColumnWidth,
  );
}
