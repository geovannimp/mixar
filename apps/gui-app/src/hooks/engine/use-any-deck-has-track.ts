import { useEngineStore } from "@/stores/engine-store";

export function useAnyDeckHasTrack(): boolean {
  return useEngineStore((state) => Boolean(state.status?.decks.some((deck) => deck.track)));
}
