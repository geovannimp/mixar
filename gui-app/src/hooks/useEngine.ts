import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import type { EngineStatus } from "../types";

export function useEngine() {
  const [status, setStatus] = useState<EngineStatus | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const refreshStatus = useCallback(async () => {
    const next = await invoke<EngineStatus>("get_status");
    setStatus(next);
  }, []);

  useEffect(() => {
    refreshStatus().catch((err: unknown) => {
      setError(String(err));
    });
  }, [refreshStatus]);

  const runAction = useCallback(
    async (action: () => Promise<void>) => {
      setBusy(true);
      setError(null);
      try {
        await action();
        await refreshStatus();
      } catch (err) {
        setError(String(err));
      } finally {
        setBusy(false);
      }
    },
    [refreshStatus],
  );

  const toggleEngine = useCallback(async () => {
    await runAction(async () => {
      if (status?.running) {
        await invoke("stop_engine");
      } else {
        await invoke("start_engine");
      }
    });
  }, [runAction, status?.running]);

  const loadTrack = useCallback(
    async (deckId: number, path: string) => {
      await runAction(async () => {
        await invoke("load_track", { deckId, path });
      });
    },
    [runAction],
  );

  const pickTrack = useCallback(
    async (deckId: number) => {
      const selected = await open({
        multiple: false,
        filters: [
          {
            name: "Audio",
            extensions: ["wav", "mp3", "flac", "ogg", "aiff", "aif"],
          },
        ],
      });
      if (typeof selected === "string") {
        await loadTrack(deckId, selected);
      }
    },
    [loadTrack],
  );

  const loadSample = useCallback(
    async (deckId: number) => {
      const samplePath = await invoke<string | null>("sample_track_path");
      if (!samplePath) {
        setError("Sample track not found. Use Load file instead.");
        return;
      }
      await loadTrack(deckId, samplePath);
    },
    [loadTrack],
  );

  const loadLibraryTrackToDeck = useCallback(
    async (deckId: number, trackId: string) => {
      await runAction(async () => {
        await invoke("load_library_track_to_deck", { deckId, trackId });
      });
    },
    [runAction],
  );

  const playDeck = useCallback(
    async (deckId: number) => {
      await runAction(async () => {
        await invoke("play_deck", { deckId });
      });
    },
    [runAction],
  );

  const pauseDeck = useCallback(
    async (deckId: number) => {
      await runAction(async () => {
        await invoke("pause_deck", { deckId });
      });
    },
    [runAction],
  );

  return {
    status,
    error,
    busy,
    toggleEngine,
    loadLibraryTrackToDeck,
    pickTrack,
    loadSample,
    playDeck,
    pauseDeck,
  };
}
