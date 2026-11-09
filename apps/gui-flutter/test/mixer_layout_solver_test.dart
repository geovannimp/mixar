import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/widgets/mixer/fx_bar.dart';
import 'package:gui_flutter/widgets/mixer/layout/mixer_layout_config.dart';
import 'package:gui_flutter/widgets/mixer/layout/mixer_layout_solver.dart';
import 'package:gui_flutter/widgets/mixer/layout/mixer_slot.dart';

const double _band = MixerPaneMetrics.bandDefault;

MixerLayoutConfig _config({
  MixerLayoutKind kind = MixerLayoutKind.waveformTop,
  int numDecks = 2,
  bool showWaveform = true,
  bool showMixer = true,
  bool showCrossfader = true,
  bool showFxBar = true,
  bool preferDockedLibrary = true,
}) => MixerLayoutConfig(
  kind: kind,
  numDecks: numDecks,
  showWaveform: showWaveform,
  showMixer: showMixer,
  showCrossfader: showCrossfader,
  showFxBar: showFxBar,
  preferDockedLibrary: preferDockedLibrary,
  mixerColumnWidth: 196,
  laneColumnWidth: numDecks == 4
      ? MixerPaneMetrics.laneWidthFour
      : MixerPaneMetrics.laneWidthTwo,
);

/// Solves for a body of [height] where the top band asks for its default 160.
MixerLayoutSolution _solve(double height, MixerLayoutConfig config) {
  final avail = height - (config.showFxBar ? FxBar.height : 0);
  final bandFraction = avail > 0 ? _band / avail : 0.0;
  return solveMixerLayout(
    height: height,
    config: config,
    bandFraction: bandFraction,
  );
}

double _avail(MixerLayoutConfig config, double height) =>
    height - (config.showFxBar ? FxBar.height : 0);

double? _topPx(MixerLayoutSolution s, double avail) =>
    s.topFraction == null ? null : s.topFraction! * avail;

double? _bottomPx(MixerLayoutSolution s, double avail) =>
    s.bottomFraction == null ? null : s.bottomFraction! * avail;

void _expectSumsToOne(MixerLayoutSolution s) {
  final sum =
      (s.topFraction ?? 0) + s.contentFraction + (s.bottomFraction ?? 0);
  expect(sum, closeTo(1, 1e-9));
  expect(s.contentFraction, greaterThan(0));
}

void main() {
  test('top band, two decks at 760: band 160, content 410, library 162', () {
    final config = _config();
    final s = _solve(760, config);
    final avail = _avail(config, 760);

    expect(_topPx(s, avail), closeTo(160, 1e-9));
    expect(s.contentFraction * avail, closeTo(410, 1e-9));
    expect(_bottomPx(s, avail), closeTo(162, 1e-9));
    expect(s.libraryDocked, isTrue);
    expect(s.barVisible, isTrue);
    _expectSumsToOne(s);
  });

  test('top band, four decks at 760: no room for the library, drawer', () {
    final config = _config(numDecks: 4);
    final s = _solve(760, config);
    final avail = _avail(config, 760);

    expect(_topPx(s, avail), closeTo(160, 1e-9));
    expect(s.contentFraction * avail, closeTo(572, 1e-9));
    expect(s.bottomFraction, isNull);
    expect(s.libraryDocked, isFalse);
    _expectSumsToOne(s);
  });

  test('top band, four decks at 728: content 540', () {
    final config = _config(numDecks: 4);
    final s = _solve(728, config);
    final avail = _avail(config, 728);

    expect(s.contentFraction * avail, closeTo(540, 1e-9));
    expect(_topPx(s, avail), closeTo(160, 1e-9));
    expect(s.bottomFraction, isNull);
    expect(s.libraryDocked, isFalse);
    _expectSumsToOne(s);
  });

  test('top band, four decks at 400: the band hides, content takes over', () {
    final config = _config(numDecks: 4);
    final s = _solve(400, config);
    final avail = _avail(config, 400);

    expect(s.topFraction, isNull);
    expect(s.contentFraction * avail, closeTo(372, 1e-9));
    expect(s.bottomFraction, isNull);
    expect(s.libraryDocked, isFalse);
    _expectSumsToOne(s);
  });

  test(
    'top band, two decks at 600: band holds, library goes to the drawer',
    () {
      final config = _config();
      final s = _solve(600, config);
      final avail = _avail(config, 600);

      expect(_topPx(s, avail), closeTo(160, 1e-9));
      expect(s.contentFraction * avail, closeTo(412, 1e-9));
      expect(s.bottomFraction, isNull);
      expect(s.libraryDocked, isFalse);
      _expectSumsToOne(s);
    },
  );

  test('bottom band, two decks at 760: library docks under the band', () {
    final config = _config(kind: MixerLayoutKind.waveformBottom);
    final s = _solve(760, config);
    final avail = _avail(config, 760);

    expect(s.topFraction, isNull);
    expect(s.contentFraction * avail, closeTo(410, 1e-9));
    expect(_bottomPx(s, avail), closeTo(322, 1e-9));
    expect(s.libraryDocked, isTrue);
    _expectSumsToOne(s);
  });

  test('bottom band, four decks at 760: the library does not fit', () {
    final config = _config(kind: MixerLayoutKind.waveformBottom, numDecks: 4);
    final s = _solve(760, config);
    final avail = _avail(config, 760);

    expect(s.contentFraction * avail, closeTo(572, 1e-9));
    expect(_bottomPx(s, avail), closeTo(160, 1e-9));
    expect(s.libraryDocked, isFalse);
    _expectSumsToOne(s);
  });

  test('bottom band, two decks at 400: the band yields before the content', () {
    final config = _config(kind: MixerLayoutKind.waveformBottom);
    final s = _solve(400, config);
    final avail = _avail(config, 400);

    expect(s.bottomFraction, isNull);
    expect(s.contentFraction * avail, closeTo(372, 1e-9));
    expect(s.libraryDocked, isFalse);
    _expectSumsToOne(s);
  });

  test('a drawer-only library never docks; the content takes the rest', () {
    final config = _config(preferDockedLibrary: false);
    final s = _solve(760, config);
    final avail = _avail(config, 760);

    expect(s.libraryDocked, isFalse);
    expect(_topPx(s, avail), closeTo(160, 1e-9));
    expect(s.contentFraction * avail, closeTo(572, 1e-9));
    expect(s.bottomFraction, isNull);
    _expectSumsToOne(s);
  });

  test('without the FX bar the splits use the whole height', () {
    final config = _config(showFxBar: false);
    final s = _solve(760, config);

    expect(s.barVisible, isFalse);
    expect(_topPx(s, 760), closeTo(160, 1e-9));
    expect(s.contentFraction * 760, closeTo(410, 1e-9));
    expect(_bottomPx(s, 760), closeTo(190, 1e-9));
    expect(s.libraryDocked, isTrue);
    _expectSumsToOne(s);
  });

  test('the hidden waveform frees the band for the docked library', () {
    final config = _config(showWaveform: false);
    final s = _solve(760, config);
    final avail = _avail(config, 760);

    expect(s.topFraction, isNull);
    expect(s.contentFraction * avail, closeTo(410, 1e-9));
    expect(_bottomPx(s, avail), closeTo(322, 1e-9));
    expect(s.libraryDocked, isTrue);
    _expectSumsToOne(s);
  });

  test('a body with no space left hides every slot but the content row', () {
    final s = solveMixerLayout(
      height: 0,
      config: _config(),
      bandFraction: 0.22,
    );

    expect(s.topFraction, isNull);
    expect(s.bottomFraction, isNull);
    expect(s.contentFraction, 1);
    // The bar slot still follows its switch, so the page and the plan agree.
    expect(s.barVisible, isTrue);
    _expectSumsToOne(s);
  });

  test('a body shorter than the FX bar keeps the bar switch as it is', () {
    final s = solveMixerLayout(
      height: FxBar.height - 1,
      config: _config(showFxBar: false),
      bandFraction: 0.22,
    );

    expect(s.barVisible, isFalse);
    expect(s.contentFraction, 1);
    _expectSumsToOne(s);
  });
}
