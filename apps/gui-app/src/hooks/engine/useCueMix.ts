import { useEngineStore } from "@/stores/engineStore";

export function useCueMix(): number {
  return useEngineStore((state) => state.status?.cue_mix ?? 0);
}
