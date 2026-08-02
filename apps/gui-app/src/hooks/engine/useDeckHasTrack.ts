import { useEngineStore } from "@/stores/engineStore";

export function useDeckHasTrack(deckId: number): boolean {
  return useEngineStore((state) => Boolean(state.status?.decks[deckId]?.track));
}
