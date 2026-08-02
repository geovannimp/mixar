import type { SamplerSlotInfo } from "@/types";
import { EMPTY_SAMPLER_SLOTS } from "@/stores/default-sampler";
import { useEngineStore } from "@/stores/engine-store";

export function useSamplerSlots(deckId: number): SamplerSlotInfo[] {
  return useEngineStore(
    (state) => state.status?.sampler?.deck_slots[deckId] ?? EMPTY_SAMPLER_SLOTS,
  );
}
