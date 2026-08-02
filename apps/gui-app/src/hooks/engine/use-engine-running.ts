import { useEngineStore } from "@/stores/engine-store";

export function useEngineRunning(): boolean {
  return useEngineStore((state) => Boolean(state.status?.running));
}
