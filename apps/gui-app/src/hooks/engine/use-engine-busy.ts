import { useEngineStore } from "@/stores/engine-store";

export function useEngineBusy(): boolean {
  return useEngineStore((state) => state.starting || state.busyDecks[0] || state.busyDecks[1]);
}
