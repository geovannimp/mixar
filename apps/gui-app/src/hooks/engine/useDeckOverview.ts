import { useShallow } from "zustand/react/shallow";
import { getDefaultDeck } from "@/stores/defaultDeck";
import { useEngineStore } from "@/stores/engineStore";

/** Overview chrome — excludes HF position; time/playhead leaves subscribe separately. */
export function useDeckOverview(deckId: number) {
  return useEngineStore(
    useShallow((state) => {
      const deck = state.status?.decks[deckId] ?? getDefaultDeck(deckId);
      return {
        track_id: deck.track_id,
        track: deck.track,
        playing: deck.playing,
        speed: deck.speed,
        duration_ms: deck.duration_ms,
        hot_cues: deck.hot_cues,
      };
    }),
  );
}
