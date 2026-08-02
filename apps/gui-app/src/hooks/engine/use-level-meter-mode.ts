import type { LevelMeterMode } from "@/types";
import { useEngineStore } from "@/stores/engine-store";

export function useLevelMeterMode(): LevelMeterMode {
  return useEngineStore((state) => state.levelMeterMode);
}
