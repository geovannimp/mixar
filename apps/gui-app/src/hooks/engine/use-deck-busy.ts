import { useEngineStore } from "@/stores/engine-store";

export function useDeckBusy(deckId: number): boolean {
  return useEngineStore((state) => state.busyDecks[deckId] ?? false);
}
