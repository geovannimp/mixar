/// Top-level deck performance surface modes (vertical Pads / Loop / Grid / Jog tabs).
///
/// Distinct from [PadMode] (Hot Cue / Roll / … inside the Pads surface).
enum DeckPerformanceMode { pads, loop, grid, jog }

const kDeckPerformanceModes = <DeckPerformanceMode>[
  DeckPerformanceMode.pads,
  DeckPerformanceMode.loop,
  DeckPerformanceMode.grid,
  DeckPerformanceMode.jog,
];

String deckPerformanceModeLabel(DeckPerformanceMode mode) => switch (mode) {
  DeckPerformanceMode.pads => 'Pads',
  DeckPerformanceMode.loop => 'Loop',
  DeckPerformanceMode.grid => 'Grid',
  DeckPerformanceMode.jog => 'Jog',
};
