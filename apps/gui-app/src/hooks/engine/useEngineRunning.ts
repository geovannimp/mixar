import { useEngineStore } from "@/stores/engineStore";

export function useEngineRunning(): boolean {
  return useEngineStore((state) => Boolean(state.status?.running));
}
