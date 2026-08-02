import { useEngineStore } from "@/stores/engineStore";

export function useCrossfader(): number {
  return useEngineStore((state) => state.status?.crossfader ?? 0.5);
}
