import type { SamplerBankInfo, SamplerSlotInfo, SamplerStatus } from "@/types";

export const EMPTY_SAMPLER_SLOTS: SamplerSlotInfo[] = Array.from({ length: 8 }, () => ({
  label: null,
  track_id: null,
  path: null,
  duration_ms: null,
}));

export const EMPTY_SAMPLER_BANKS: SamplerBankInfo[] = [];

export const DEFAULT_SAMPLER_STATUS: SamplerStatus = {
  banks: EMPTY_SAMPLER_BANKS,
  active_bank_id: null,
  active_bank_name: null,
  bank_play_mode: null,
  deck_slots: [EMPTY_SAMPLER_SLOTS, EMPTY_SAMPLER_SLOTS],
  effective_play_modes: ["oneshot", "oneshot"],
};
