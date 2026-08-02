import { useShallow } from "zustand/react/shallow";
import { getDefaultDeck } from "@/stores/defaultDeck";
import { useEngineStore } from "@/stores/engineStore";

export function useDeckTransport(deckId: number) {
  return useEngineStore(
    useShallow((state) => {
      const deck = state.status?.decks[deckId] ?? getDefaultDeck(deckId);
      return {
        position_ms: deck.position_ms,
        duration_ms: deck.duration_ms,
        playing: deck.playing,
      };
    }),
  );
}
