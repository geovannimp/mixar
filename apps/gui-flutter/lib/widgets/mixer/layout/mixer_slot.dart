/// Slots in the mixer page's main pane stack, top to bottom.
enum MixerMainSlot { top, content, bottom, bar }

/// Slots in the content row, left to right.
enum MixerContentSlot { left, center, right }

/// Pane ids for the two stacks; the page's controllers key their entries by
/// these, the widget's builders switch on them.
abstract final class MixerPaneIds {
  static const top = 'mixer.top';
  static const content = 'mixer.content';
  static const bottom = 'mixer.bottom';
  static const bar = 'mixer.bar';
  static const left = 'mixer.left';
  static const center = 'mixer.center';
  static const right = 'mixer.right';
}

/// Spacing vocabulary shared by the rectangles, the solver and the plan.
///
/// The deck cards inset their body by [deckInset] and used to sit one
/// [slotGap] off the mixer column; the strip bleeds the sum of the two so its
/// rule reaches the deck card borders. No other module hardcodes these.
abstract final class MixerPaneMetrics {
  /// Deck body inset; the stacked split stops at the panel edges.
  static const double deckInset = 10;

  /// Gap between the mixer column and each deck column.
  static const double slotGap = 8;

  /// Space the mixer strip bleeds so its rule spans the deck gutters.
  static const double mixerBleed = deckInset + slotGap;

  /// Waveform band's starting height.
  static const double bandDefault = 160;

  /// Waveform band's floor; below it the band hides.
  static const double bandMin = 110;

  /// Fewest pixels a docked library is worth; under it the library moves to
  /// the drawer.
  static const double libraryMin = 100;

  /// Content row height the layout aims for, two and four decks.
  static const double contentPreferredTwo = 410;
  static const double contentPreferredFour = 560;

  /// Content row height the layout accepts before the band yields.
  static const double contentFloorTwo = 300;
  static const double contentFloorFour = 380;

  /// Centre column thickness for the middle waveform positions.
  static const double laneWidthTwo = 320;
  static const double laneWidthFour = 388;
}
