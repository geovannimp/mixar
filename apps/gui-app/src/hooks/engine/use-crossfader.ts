import { useEngineStore } from "@/stores/engine-store";

export function useCrossfader(): number {
  return useEngineStore((state) => state.status?.crossfader ?? 0.5);
}
