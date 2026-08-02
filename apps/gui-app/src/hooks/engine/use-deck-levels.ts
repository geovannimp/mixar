import { ZERO_DECK_LEVELS } from "@/types";
import { useEngineStore } from "@/stores/engine-store";

export function useDeckLevels(deckId: number) {
  return useEngineStore((state) => state.status?.decks[deckId]?.levels ?? ZERO_DECK_LEVELS);
}
