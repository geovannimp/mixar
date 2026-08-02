import { useShallow } from "zustand/react/shallow";
import { getDefaultDeck } from "@/stores/defaultDeck";
import { useEngineStore } from "@/stores/engineStore";

/** Mixer knobs/faders — excludes HF levels so VU ticks do not re-render EQ/volume. */
export function useDeckMixerChannel(deckId: number) {
  return useEngineStore(
    useShallow((state) => {
      const deck = state.status?.decks[deckId] ?? getDefaultDeck(deckId);
      return {
        volume: deck.volume,
        eq: deck.eq,
        filter_db: deck.filter_db,
        gain_trim_db: deck.gain_trim_db,
        headphone_cue: deck.headphone_cue,
      };
    }),
  );
}
