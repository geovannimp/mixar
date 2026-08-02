import type { SamplerBankInfo } from "@/types";
import { EMPTY_SAMPLER_BANKS } from "@/stores/defaultSampler";
import { useEngineStore } from "@/stores/engineStore";

export function useSamplerBanks(): SamplerBankInfo[] {
  return useEngineStore((state) => state.status?.sampler?.banks ?? EMPTY_SAMPLER_BANKS);
}
