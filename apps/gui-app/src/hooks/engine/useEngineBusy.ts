import { useEngineStore } from "@/stores/engineStore";

export function useEngineBusy(): boolean {
  return useEngineStore((state) => state.starting || state.busyDecks[0] || state.busyDecks[1]);
}
