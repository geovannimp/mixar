import { useCallback, useEffect, useSyncExternalStore } from "react";
import { invoke } from "@tauri-apps/api/core";
import { toastManager } from "@/components/ui/toast";
import type { AppSettings } from "@/types";

type SettingsSnapshot = {
  settings: AppSettings | null;
  error: string | null;
  busy: boolean;
  saved: boolean;
};

let snapshot: SettingsSnapshot = {
  settings: null,
  error: null,
  busy: false,
  saved: false,
};

const listeners = new Set<() => void>();
let loadPromise: Promise<AppSettings> | null = null;

function emit() {
  for (const listener of listeners) {
    listener();
  }
}

function patchSnapshot(partial: Partial<SettingsSnapshot>) {
  snapshot = { ...snapshot, ...partial };
  emit();
}

function subscribe(onStoreChange: () => void) {
  listeners.add(onStoreChange);
  return () => {
    listeners.delete(onStoreChange);
  };
}

function getSnapshot() {
  return snapshot;
}

async function loadSettings(): Promise<AppSettings> {
  if (snapshot.settings != null) {
    return snapshot.settings;
  }
  if (!loadPromise) {
    loadPromise = invoke<AppSettings>("get_settings")
      .then((next) => {
        patchSnapshot({ settings: next, error: null });
        return next;
      })
      .catch((err: unknown) => {
        loadPromise = null;
        patchSnapshot({ error: String(err) });
        throw err;
      });
  }
  return loadPromise;
}

/** Shared across all `useSettings()` callers so saves update decks/mixer immediately. */
export function useSettings() {
  const snap = useSyncExternalStore(subscribe, getSnapshot, getSnapshot);

  const refresh = useCallback(async () => {
    loadPromise = null;
    const next = await invoke<AppSettings>("get_settings");
    patchSnapshot({ settings: next, error: null });
    return next;
  }, []);

  useEffect(() => {
    void loadSettings().catch(() => {
      /* error already in snapshot */
    });
  }, []);

  const save = useCallback(async (next: AppSettings) => {
    patchSnapshot({ busy: true, error: null, saved: false });
    try {
      const updated = await invoke<AppSettings>("save_settings", { settings: next });
      patchSnapshot({ settings: updated, saved: true });
    } catch (err) {
      const message = String(err);
      patchSnapshot({ error: message });
      toastManager.add({
        id: "settings-restart-error",
        title: `Engine restart failed: ${message}`,
        type: "error",
      });
    } finally {
      patchSnapshot({ busy: false });
    }
  }, []);

  return {
    settings: snap.settings,
    error: snap.error,
    busy: snap.busy,
    saved: snap.saved,
    save,
    refresh,
  };
}
