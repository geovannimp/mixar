import type { LevelMeterMode } from "@/types";
import { useEngineStore } from "@/stores/engineStore";

export function useLevelMeterMode(): LevelMeterMode {
  return useEngineStore((state) => state.levelMeterMode);
}
