import { useShallow } from "zustand/react/shallow";
import { useEngineStore } from "@/stores/engineStore";

export function useEngineHeaderInfo() {
  return useEngineStore(
    useShallow((state) => ({
      running: Boolean(state.status?.running),
      backend: state.status?.backend ?? "",
      sampleRate: state.status?.sample_rate ?? 0,
    })),
  );
}
