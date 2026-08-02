import { useEngineStore } from "@/stores/engine-store";

export function useDeckPosition(deckId: number): number {
  return useEngineStore((state) => state.status?.decks[deckId]?.position_ms ?? 0);
}
