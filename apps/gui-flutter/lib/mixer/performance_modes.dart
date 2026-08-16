/// Top-level deck performance surface modes (vertical Pads / Loop / Jog tabs).
///
/// Distinct from [PadMode] (Hot Cue / Roll / … inside the Pads surface).
enum DeckPerformanceMode { pads, loop, jog }

const kDeckPerformanceModes = <DeckPerformanceMode>[
  DeckPerformanceMode.pads,
  DeckPerformanceMode.loop,
  DeckPerformanceMode.jog,
];

String deckPerformanceModeLabel(DeckPerformanceMode mode) => switch (mode) {
  DeckPerformanceMode.pads => 'Pads',
  DeckPerformanceMode.loop => 'Loop',
  DeckPerformanceMode.jog => 'Jog',
};
