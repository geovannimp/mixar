/// Top-level deck performance surface modes (vertical Pads / Loop tabs).
///
/// Distinct from [PadMode] (Hot Cue / Roll / … inside the Pads surface).
enum DeckPerformanceMode { pads, loop }

const kDeckPerformanceModes = <DeckPerformanceMode>[
  DeckPerformanceMode.pads,
  DeckPerformanceMode.loop,
];

String deckPerformanceModeLabel(DeckPerformanceMode mode) => switch (mode) {
  DeckPerformanceMode.pads => 'Pads',
  DeckPerformanceMode.loop => 'Loop',
};
