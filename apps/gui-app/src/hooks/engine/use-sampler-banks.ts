import type { SamplerBankInfo } from "@/types";
import { EMPTY_SAMPLER_BANKS } from "@/stores/default-sampler";
import { useEngineStore } from "@/stores/engine-store";

export function useSamplerBanks(): SamplerBankInfo[] {
  return useEngineStore((state) => state.status?.sampler?.banks ?? EMPTY_SAMPLER_BANKS);
}
