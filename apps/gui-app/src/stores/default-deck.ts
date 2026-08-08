import { DEFAULT_DECK_EQ, ZERO_DECK_LEVELS, type DeckStatus } from "@/types";
import { DEFAULT_TEMPO_RANGE } from "@/lib/bus-settings";

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
  speed: 0.5,
  tempo_range: DEFAULT_TEMPO_RANGE,
  eq: DEFAULT_DECK_EQ,
  position_ms: null,
  duration_ms: null,
  cue_point_ms: null,
  quantize: true,
  hot_cues: EMPTY_HOT_CUES,
  saved_loops: EMPTY_SAVED_LOOPS,
  active_loop: null,
  filter: 0.5,
  gain_trim: 0.5,
  loudness_lufs: null,
  auto_gain_db: 0,
  sync_mode: "off",
  is_master: true,
  pad_mode: "hot_cue",
  headphone_cue: false,
  active_sampler_bank_id: null,
  top_jog_mode: "vinyl",
  outer_jog_mode: "pitch_bend",
  jog_touching: false,
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
