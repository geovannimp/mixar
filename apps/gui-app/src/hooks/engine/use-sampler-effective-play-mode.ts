import type { SamplerPlayMode } from "@/types";
import { useEngineStore } from "@/stores/engine-store";

export function useSamplerEffectivePlayMode(deckId: number): SamplerPlayMode {
  return useEngineStore(
    (state) => state.status?.sampler?.effective_play_modes[deckId] ?? "oneshot",
  );
}
