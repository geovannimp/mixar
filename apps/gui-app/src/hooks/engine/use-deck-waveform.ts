import { useShallow } from "zustand/react/shallow";
import { getDefaultDeck } from "@/stores/default-deck";
import { useEngineStore } from "@/stores/engine-store";

/** Waveform chrome — excludes HF position; playhead leaves subscribe via useDeckPosition. */
export function useDeckWaveform(deckId: number) {
  return useEngineStore(
    useShallow((state) => {
      const deck = state.status?.decks[deckId] ?? getDefaultDeck(deckId);
      return {
        id: deck.id,
        track: deck.track,
        track_id: deck.track_id,
        playing: deck.playing,
        speed: deck.speed,
        eq: deck.eq,
        hot_cues: deck.hot_cues,
        active_loop: deck.active_loop,
        duration_ms: deck.duration_ms,
      };
    }),
  );
}
