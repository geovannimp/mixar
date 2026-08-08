import { useShallow } from "zustand/react/shallow";
import { normToSpeedRatio } from "@/lib/format";
import { getDefaultDeck } from "@/stores/default-deck";
import { useEngineStore } from "@/stores/engine-store";

/** Overview chrome — excludes HF position; time/playhead leaves subscribe separately. */
export function useDeckOverview(deckId: number) {
  return useEngineStore(
    useShallow((state) => {
      const deck = state.status?.decks[deckId] ?? getDefaultDeck(deckId);
      return {
        track_id: deck.track_id,
        track: deck.track,
        playing: deck.playing,
        speed: normToSpeedRatio(deck.speed, deck.tempo_range),
        duration_ms: deck.duration_ms,
        hot_cues: deck.hot_cues,
      };
    }),
  );
}
