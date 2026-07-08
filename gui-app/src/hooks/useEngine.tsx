import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useState,
  type ReactNode,
} from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import type { DeckEq, DeckStatus, EngineStatus } from "../types";

export interface EngineContextValue {
  status: EngineStatus | null;
  error: string | null;
  busy: boolean;
  toggleEngine: () => Promise<void>;
  loadLibraryTrackToDeck: (deckId: number, trackId: string) => Promise<void>;
  loadPathToDeck: (deckId: number, path: string) => Promise<void>;
  pickTrack: (deckId: number) => Promise<void>;
  playDeck: (deckId: number) => Promise<void>;
  pauseDeck: (deckId: number) => Promise<void>;
  setDeckVolume: (deckId: number, volume: number) => Promise<void>;
  setDeckEq: (deckId: number, eq: DeckEq) => Promise<void>;
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

  const anyDeckPlaying = status?.decks.some((deck) => deck.playing) ?? false;

  useEffect(() => {
    if (!status?.running || !anyDeckPlaying) {
      return;
    }

    const intervalId = window.setInterval(() => {
      refreshStatus().catch((err: unknown) => {
        setError(String(err));
      });
    }, 100);

    return () => window.clearInterval(intervalId);
  }, [status?.running, anyDeckPlaying, refreshStatus]);

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

  const loadLibraryTrackToDeck = useCallback(
    async (deckId: number, trackId: string) => {
      await runAction(async () => {
        await invoke("load_library_track_to_deck", { deckId, trackId });
      });
    },
    [runAction],
  );

  const loadPathToDeck = useCallback(
    async (deckId: number, path: string) => {
      await runAction(async () => {
        await invoke("load_path_to_deck", { deckId, path });
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
      setError(null);
      try {
        const updated = await invoke<DeckStatus>("set_deck_volume", {
          deckId,
          volume,
        });
        setStatus((current) => {
          if (!current) {
            return current;
          }
          return {
            ...current,
            decks: current.decks.map((deck) =>
              deck.id === deckId ? updated : deck,
            ),
          };
        });
      } catch (err) {
        setError(String(err));
      }
    },
    [],
  );

  const setDeckEq = useCallback(async (deckId: number, eq: DeckEq) => {
    setError(null);
    try {
      const updated = await invoke<DeckStatus>("set_deck_eq", {
        deckId,
        low: eq.low,
        mid: eq.mid,
        high: eq.high,
      });
      setStatus((current) => {
        if (!current) {
          return current;
        }
        return {
          ...current,
          decks: current.decks.map((deck) =>
            deck.id === deckId ? updated : deck,
          ),
        };
      });
    } catch (err) {
      setError(String(err));
    }
  }, []);

  const setCrossfader = useCallback(async (position: number) => {
    setError(null);
    try {
      const updated = await invoke<EngineStatus>("set_crossfader", {
        crossfader: position,
      });
      setStatus(updated);
    } catch (err) {
      setError(String(err));
    }
  }, []);

  return {
    status,
    error,
    busy,
    toggleEngine,
    loadLibraryTrackToDeck,
    loadPathToDeck,
    pickTrack,
    playDeck,
    pauseDeck,
    setDeckVolume,
    setDeckEq,
    setCrossfader,
  };
}
