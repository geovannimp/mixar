import type { SamplerSlotInfo } from "@/types";
import { EMPTY_SAMPLER_SLOTS } from "@/stores/defaultSampler";
import { useEngineStore } from "@/stores/engineStore";

export function useSamplerSlots(deckId: number): SamplerSlotInfo[] {
  return useEngineStore(
    (state) => state.status?.sampler?.deck_slots[deckId] ?? EMPTY_SAMPLER_SLOTS,
  );
}
