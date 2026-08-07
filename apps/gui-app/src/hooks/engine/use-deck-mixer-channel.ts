import { useShallow } from "zustand/react/shallow";
import { getDefaultDeck } from "@/stores/default-deck";
import { useEngineStore } from "@/stores/engine-store";

/** Mixer knobs/faders — excludes HF levels so VU ticks do not re-render EQ/volume. */
export function useDeckMixerChannel(deckId: number) {
  return useEngineStore(
    useShallow((state) => {
      const deck = state.status?.decks[deckId] ?? getDefaultDeck(deckId);
      return {
        volume: deck.volume,
        eq: deck.eq,
        filter: deck.filter,
        gain_trim: deck.gain_trim,
        headphone_cue: deck.headphone_cue,
      };
    }),
  );
}
