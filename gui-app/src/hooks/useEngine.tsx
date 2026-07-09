import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useRef,
  useState,
  type ReactNode,
} from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import { toastManager } from "@/components/ui/toast";
import {
  applyEngineEvent,
  ENGINE_EVENT,
  type EngineEvent,
} from "../lib/engineEvents";
import type { DeckEq, EngineStatus } from "../types";

const ENGINE_ERROR_TOAST_ID = "engine-error";

function reportEngineError(message: string) {
  toastManager.add({
    id: ENGINE_ERROR_TOAST_ID,
    title: message,
    type: "error",
  });
}

export interface EngineContextValue {
  status: EngineStatus | null;
  busy: boolean;
  ensureEngineRunning: () => Promise<void>;
  loadLibraryTrackToDeck: (deckId: number, trackId: string) => Promise<void>;
  loadPathToDeck: (deckId: number, path: string) => Promise<void>;
  pickTrack: (deckId: number) => Promise<void>;
  playDeck: (deckId: number) => Promise<void>;
  pauseDeck: (deckId: number) => Promise<void>;
  setDeckVolume: (deckId: number, volume: number) => Promise<void>;
  setDeckEq: (deckId: number, eq: DeckEq) => Promise<void>;
  setDeckSpeed: (deckId: number, speed: number) => Promise<void>;
  setCrossfader: (position: number) => Promise<void>;
}

const EngineContext = createContext<EngineContextValue | null>(null);

export function EngineProvider({ children }: { children: ReactNode }) {
  const value = useEngineState();
  return (
    <EngineContext.Provider value={value}>{children}</EngineContext.Provider>
  );
}

export function useEngine(): EngineContextValue {
  const context = useContext(EngineContext);
  if (!context) {
    throw new Error("useEngine must be used within EngineProvider");
  }
  return context;
}

function useEngineState(): EngineContextValue {
  const [status, setStatus] = useState<EngineStatus | null>(null);
  const [busy, setBusy] = useState(false);
  const startingRef = useRef(false);
  const revisionRef = useRef(0);

  const applyEvent = useCallback((event: EngineEvent) => {
    if (event.type === "error") {
      reportEngineError(event.message);
      return;
    }
    if (event.type === "notice") {
      toastManager.add({ title: event.message, type: "info" });
      return;
    }

    setStatus((current) => {
      const next = applyEngineEvent(current, event, revisionRef.current);
      revisionRef.current = next.revision;
      return next.status;
    });
  }, []);

  const refreshStatus = useCallback(async () => {
    const next = await invoke<EngineStatus>("get_status");
    setStatus(next);
  }, []);

  useEffect(() => {
    refreshStatus().catch((err: unknown) => {
      reportEngineError(String(err));
    });
  }, [refreshStatus]);

  useEffect(() => {
    let unlisten: (() => void) | undefined;

    listen<EngineEvent>(ENGINE_EVENT, (event) => {
      applyEvent(event.payload);
    })
      .then((dispose) => {
        unlisten = dispose;
      })
      .catch((err: unknown) => {
        reportEngineError(String(err));
      });

    return () => {
      unlisten?.();
    };
  }, [applyEvent]);

  const anyDeckPlaying = status?.decks.some((deck) => deck.playing) ?? false;

  useEffect(() => {
    if (!status?.running || !anyDeckPlaying) {
      return;
    }

    const intervalId = window.setInterval(() => {
      refreshStatus().catch((err: unknown) => {
        reportEngineError(String(err));
      });
    }, 100);

    return () => window.clearInterval(intervalId);
  }, [status?.running, anyDeckPlaying, refreshStatus]);

  const runAction = useCallback(async (action: () => Promise<void>) => {
    setBusy(true);
    try {
      await action();
    } catch (err) {
      reportEngineError(String(err));
    } finally {
      setBusy(false);
    }
  }, []);

  const ensureEngineRunning = useCallback(async () => {
    if (status?.running || startingRef.current) {
      return;
    }

    startingRef.current = true;
    setBusy(true);

    try {
      await toastManager.promise(
        (async () => {
          await invoke("start_engine");
        })(),
        {
          loading: {
            title: "Starting engine…",
            type: "loading",
          },
          success: {
            title: "Engine running",
            type: "success",
          },
          error: (err: unknown) => ({
            title: err instanceof Error ? err.message : String(err),
            type: "error",
          }),
        },
      );
    } finally {
      startingRef.current = false;
      setBusy(false);
    }
  }, [status?.running]);

  const loadPathToDeck = useCallback(
    async (deckId: number, path: string) => {
      await runAction(async () => {
        await invoke("load_path_to_deck", { deckId, path });
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
        await loadPathToDeck(deckId, selected);
      }
    },
    [loadPathToDeck],
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

  const setDeckVolume = useCallback(
    async (deckId: number, volume: number) => {
      try {
        await invoke("set_deck_volume", { deckId, volume });
      } catch (err) {
        reportEngineError(String(err));
      }
    },
    [],
  );

  const setDeckEq = useCallback(async (deckId: number, eq: DeckEq) => {
    try {
      await invoke("set_deck_eq", {
        deckId,
        low: eq.low,
        mid: eq.mid,
        high: eq.high,
      });
    } catch (err) {
      reportEngineError(String(err));
    }
  }, []);

  const setDeckSpeed = useCallback(async (deckId: number, speed: number) => {
    try {
      await invoke("set_deck_speed", { deckId, speed });
    } catch (err) {
      reportEngineError(String(err));
    }
  }, []);

  const setCrossfader = useCallback(async (position: number) => {
    try {
      await invoke("set_crossfader", { crossfader: position });
    } catch (err) {
      reportEngineError(String(err));
    }
  }, []);

  return {
    status,
    busy,
    ensureEngineRunning,
    loadLibraryTrackToDeck,
    loadPathToDeck,
    pickTrack,
    playDeck,
    pauseDeck,
    setDeckVolume,
    setDeckEq,
    setDeckSpeed,
    setCrossfader,
  };
}
