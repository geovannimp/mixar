import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { toastManager } from "@/components/ui/toast";
import type { AppSettings } from "../types";

export function useSettings() {
  const [settings, setSettings] = useState<AppSettings | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [saved, setSaved] = useState(false);

  const refresh = useCallback(async () => {
    const next = await invoke<AppSettings>("get_settings");
    setSettings(next);
    return next;
  }, []);

  useEffect(() => {
    refresh().catch((err: unknown) => {
      setError(String(err));
    });
  }, [refresh]);

  const save = useCallback(
    async (next: AppSettings) => {
      setBusy(true);
      setError(null);
      setSaved(false);
      try {
        const updated = await invoke<AppSettings>("save_settings", { settings: next });
        setSettings(updated);
        setSaved(true);
      } catch (err) {
        const message = String(err);
        setError(message);
        toastManager.add({
          id: "settings-restart-error",
          title: `Engine restart failed: ${message}`,
          type: "error",
        });
      } finally {
        setBusy(false);
      }
    },
    [],
  );

  return {
    settings,
    error,
    busy,
    saved,
    save,
    refresh,
  };
}
