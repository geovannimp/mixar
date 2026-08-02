import { useEngineStore } from "@/stores/engine-store";

export function useCueMix(): number {
  return useEngineStore((state) => state.status?.cue_mix ?? 0);
}
