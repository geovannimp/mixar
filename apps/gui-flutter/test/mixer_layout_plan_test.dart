import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/widgets/mixer/layout/mixer_content.dart';
import 'package:gui_flutter/widgets/mixer/layout/mixer_layout_config.dart';
import 'package:gui_flutter/widgets/mixer/layout/mixer_layout_solver.dart';
import 'package:gui_flutter/widgets/mixer/layout/mixer_plan.dart';
import 'package:gui_flutter/widgets/mixer/layout/mixer_slot.dart';

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

MixerLayoutSolution _solution({
  double? top = 0.22,
  double content = 0.56,
  double? bottom = 0.22,
  bool libraryDocked = true,
  bool barVisible = true,
}) => MixerLayoutSolution(
  topFraction: top,
  contentFraction: content,
  bottomFraction: bottom,
  libraryDocked: libraryDocked,
  barVisible: barVisible,
);

MixerPlan _plan(MixerLayoutConfig config, [MixerLayoutSolution? solution]) =>
    mixerPlanFor(config, solution ?? _solution());

GroupContent _groupOf(MixerContent? content) {
  expect(content, isA<GroupContent>());
  return content! as GroupContent;
}

void main() {
  group('top band', () {
    test('fills the top slot and the mixer takes the centre column', () {
      final plan = _plan(_config());

      expect(plan.top!.content, isA<WaveformContent>());
      expect(plan.top!.minPx, MixerPaneMetrics.bandMin);
      expect(plan.content!.center!.content, isA<MixerStripContent>());
      expect(plan.content!.centerWidth, 196);
      expect(plan.content!.seam, isFalse);
      expect(plan.content!.left!.content, isA<DeckContent>());
      expect(plan.content!.right!.content, isA<DeckContent>());
      expect(plan.bottom!.content, isA<LibraryContent>());
      expect(plan.bottom!.minPx, MixerPaneMetrics.libraryMin);
      expect(plan.bar!.content, isA<FxBarContent>());
    });

    test('a hidden waveform leaves the top slot empty', () {
      final plan = _plan(_config(showWaveform: false));

      expect(plan.top, isNull);
      expect(plan.content!.center!.content, isA<MixerStripContent>());
    });

    test('an undocked library leaves the bottom slot empty', () {
      final plan = _plan(_config(), _solution(libraryDocked: false));

      expect(plan.bottom, isNull);
    });

    test('the drawer setting keeps the library out even when it fits', () {
      final plan = _plan(_config(preferDockedLibrary: false));

      expect(plan.bottom, isNull);
    });

    test('the deck facing the mixer keeps its inner inset', () {
      final plan = _plan(_config());
      final left = plan.content!.left!.content as DeckContent;
      final right = plan.content!.right!.content as DeckContent;

      expect(left.padding, const EdgeInsets.only(left: 10, right: 10));
      expect(right.padding, const EdgeInsets.only(left: 10, right: 10));
    });

    test('four decks stack in pairs with a divider inset', () {
      final plan = _plan(_config(numDecks: 4));
      final left = plan.content!.left!.content as DeckPairContent;
      final right = plan.content!.right!.content as DeckPairContent;

      expect(left.topId, 0);
      expect(left.bottomId, 2);
      expect(left.dividerPadding, const EdgeInsets.only(right: 10));
      expect(right.topId, 1);
      expect(right.bottomId, 3);
      expect(right.dividerPadding, const EdgeInsets.only(left: 10));
      expect(plan.content!.centerWidth, 196);
    });
  });

  group('bottom band', () {
    test('groups the band over the docked library', () {
      final plan = _plan(_config(kind: MixerLayoutKind.waveformBottom));
      final group = _groupOf(plan.bottom!.content);

      expect(plan.top, isNull);
      expect(group.axis, Axis.vertical);
      expect(group.children, hasLength(2));
      expect(group.children.first.content, isA<WaveformContent>());
      expect(group.children.first.fill, isFalse);
      expect(group.children.first.extent, MixerPaneMetrics.bandDefault);
      expect(group.children.last.content, isA<LibraryContent>());
      expect(group.children.last.fill, isTrue);
      expect(group.children.last.min, MixerPaneMetrics.libraryMin);
      expect(group.children.last.dividerBefore, isTrue);
      expect(plan.bottom!.minPx, MixerPaneMetrics.bandDefault);
    });

    test('holds the band alone when the library is in the drawer', () {
      final plan = _plan(
        _config(kind: MixerLayoutKind.waveformBottom),
        _solution(libraryDocked: false),
      );
      final group = _groupOf(plan.bottom!.content);

      expect(group.children, hasLength(1));
      expect(group.children.single.content, isA<WaveformContent>());
    });

    test('without the waveform the bottom slot is only the library', () {
      final plan = _plan(
        _config(kind: MixerLayoutKind.waveformBottom, showWaveform: false),
      );

      expect(plan.bottom!.content, isA<LibraryContent>());
      final undocked = _plan(
        _config(kind: MixerLayoutKind.waveformBottom, showWaveform: false),
        _solution(libraryDocked: false),
      );
      expect(undocked.bottom, isNull);
    });
  });

  group('centre column', () {
    test('horizontal lanes carry the crossfader under them', () {
      final plan = _plan(_config(kind: MixerLayoutKind.centreHorizontal));
      final group = _groupOf(plan.content!.center!.content);

      expect(plan.top, isNull);
      expect(group.axis, Axis.vertical);
      expect(group.children.first.content, isA<WaveformContent>());
      expect(group.children.first.fill, isTrue);
      expect(group.children.last.content, isA<CrossfaderContent>());
      expect(group.children.last.fill, isFalse);
      expect(plan.content!.centerWidth, MixerPaneMetrics.laneWidthTwo);
      expect(plan.content!.seam, isFalse);
    });

    test('vertical lanes put one lane per deck side by side', () {
      final plan = _plan(_config(kind: MixerLayoutKind.centreVertical));
      final group = _groupOf(plan.content!.center!.content);

      expect(group.children.first.content, isA<VerticalLanesContent>());
    });

    test('four decks widen the centre column', () {
      final plan = _plan(
        _config(kind: MixerLayoutKind.centreHorizontal, numDecks: 4),
      );

      expect(plan.content!.centerWidth, MixerPaneMetrics.laneWidthFour);
    });

    test('the crossfader switch empties the lanes group', () {
      final plan = _plan(
        _config(kind: MixerLayoutKind.centreVertical, showCrossfader: false),
      );
      final group = _groupOf(plan.content!.center!.content);

      expect(group.children, hasLength(1));
      expect(group.children.single.content, isA<VerticalLanesContent>());
    });

    test('lanes take the mixer gutters, so the decks meet them flush', () {
      final plan = _plan(_config(kind: MixerLayoutKind.centreVertical));
      final left = plan.content!.left!.content as DeckContent;
      final right = plan.content!.right!.content as DeckContent;

      expect(left.padding, const EdgeInsets.only(left: 10));
      expect(right.padding, const EdgeInsets.only(right: 10));
    });
  });

  group('no centre column', () {
    test('hiding the mixer leaves the decks on a seam', () {
      final plan = _plan(_config(showMixer: false));
      final left = plan.content!.left!.content as DeckContent;
      final right = plan.content!.right!.content as DeckContent;

      expect(plan.content!.center, isNull);
      expect(plan.content!.seam, isTrue);
      expect(plan.content!.centerWidth, 0);
      expect(left.padding, const EdgeInsets.only(left: 10));
      expect(right.padding, const EdgeInsets.only(right: 10));
    });

    test('four decks close their split at the seam edge', () {
      final plan = _plan(_config(showMixer: false, numDecks: 4));
      final left = plan.content!.left!.content as DeckPairContent;

      expect(left.dividerPadding, EdgeInsets.zero);
    });

    test('the middle placements keep a centre even with the mixer off', () {
      final plan = _plan(
        _config(kind: MixerLayoutKind.centreHorizontal, showMixer: false),
      );

      expect(plan.content!.center, isNotNull);
      expect(plan.content!.seam, isFalse);
    });
  });

  test('the FX bar switch empties the bar slot', () {
    expect(_plan(_config(showFxBar: false)).bar, isNull);
  });
}
