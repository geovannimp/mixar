import { DEFAULT_DECK_EQ, ZERO_DECK_LEVELS, type DeckStatus } from "../types";

const EMPTY_HOT_CUES: DeckStatus["hot_cues"] = [];
const EMPTY_SAVED_LOOPS: DeckStatus["saved_loops"] = [];

export const DEFAULT_DECK_A: DeckStatus = {
  id: 0,
  track: null,
  track_id: null,
  title: null,
  artist: null,
  bpm: null,
  key: null,
  playing: false,
  volume: 1,
  speed: 1,
  eq: DEFAULT_DECK_EQ,
  position_secs: null,
  duration_secs: null,
  cue_point_secs: null,
  quantize: true,
  hot_cues: EMPTY_HOT_CUES,
  saved_loops: EMPTY_SAVED_LOOPS,
  active_loop: null,
  filter_db: 0,
  gain_trim_db: 0,
  sync_mode: "off",
  is_master: true,
  pad_mode: "hot_cue",
  headphone_cue: false,
  levels: ZERO_DECK_LEVELS,
};

export const DEFAULT_DECK_B: DeckStatus = {
  ...DEFAULT_DECK_A,
  id: 1,
  is_master: false,
};

const DEFAULT_DECKS = [DEFAULT_DECK_A, DEFAULT_DECK_B] as const;

export function getDefaultDeck(deckId: number): DeckStatus {
  return DEFAULT_DECKS[deckId] ?? DEFAULT_DECK_A;
}

/** @deprecated Prefer getDefaultDeck for stable empty-deck references. */
export function createDefaultDeck(id: number): DeckStatus {
  if (id === 0) {
    return DEFAULT_DECK_A;
  }
  if (id === 1) {
    return DEFAULT_DECK_B;
  }
  return { ...DEFAULT_DECK_A, id };
}
