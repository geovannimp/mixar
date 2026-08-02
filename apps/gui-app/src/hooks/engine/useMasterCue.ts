import { useEngineStore } from "@/stores/engineStore";

export function useMasterCue(): boolean {
  return useEngineStore((state) => state.status?.master_cue ?? false);
}
