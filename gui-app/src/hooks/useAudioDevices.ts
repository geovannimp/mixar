import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { AudioDeviceSummary } from "@/types";

export function useAudioDevices(backend: string) {
  const [devices, setDevices] = useState<AudioDeviceSummary[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  const refresh = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const next = await invoke<AudioDeviceSummary[]>("list_output_devices", {
        backend,
      });
      setDevices(next);
    } catch (err) {
      setError(String(err));
      setDevices([]);
    } finally {
      setLoading(false);
    }
  }, [backend]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  return { devices, error, loading, refresh };
}
