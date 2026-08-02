import { useEngineStore } from "@/stores/engineStore";

export function useAnyDeckHasTrack(): boolean {
  return useEngineStore((state) => Boolean(state.status?.decks.some((deck) => deck.track)));
}
