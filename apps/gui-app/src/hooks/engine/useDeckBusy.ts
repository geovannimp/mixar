import { useEngineStore } from "@/stores/engineStore";

export function useDeckBusy(deckId: number): boolean {
  return useEngineStore((state) => state.busyDecks[deckId] ?? false);
}
