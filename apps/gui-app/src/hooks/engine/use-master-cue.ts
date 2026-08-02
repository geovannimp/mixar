import { useEngineStore } from "@/stores/engine-store";

export function useMasterCue(): boolean {
  return useEngineStore((state) => state.status?.master_cue ?? false);
}
