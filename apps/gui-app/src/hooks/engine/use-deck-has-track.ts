import { useEngineStore } from "@/stores/engine-store";

export function useDeckHasTrack(deckId: number): boolean {
  return useEngineStore((state) => Boolean(state.status?.decks[deckId]?.track));
}
