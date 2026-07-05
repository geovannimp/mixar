import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";

interface DeckStatus {
  id: number;
  track: string | null;
  playing: boolean;
}

interface EngineStatus {
  running: boolean;
  backend: string;
  sample_rate: number;
  decks: DeckStatus[];
}

const DECK_LABELS = ["Deck A", "Deck B"] as const;

const buttonBase =
  "rounded-lg border px-4 py-2.5 text-sm font-medium transition disabled:cursor-not-allowed disabled:opacity-45";

function fileName(path: string | null): string {
  if (!path) return "No track loaded";
  const parts = path.split(/[/\\]/);
  return parts[parts.length - 1] ?? path;
}

function statusPill(active: boolean): string {
  return active
    ? "rounded-full bg-emerald-500/15 px-2.5 py-0.5 text-xs font-semibold uppercase tracking-wide text-emerald-300"
    : "rounded-full bg-white/8 px-2.5 py-0.5 text-xs font-semibold uppercase tracking-wide text-slate-400";
}

function App() {
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

  async function runAction(action: () => Promise<void>) {
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
  }

  async function toggleEngine() {
    await runAction(async () => {
      if (status?.running) {
        await invoke("stop_engine");
      } else {
        await invoke("start_engine");
      }
    });
  }

  async function loadTrack(deckId: number, path: string) {
    await runAction(async () => {
      await invoke("load_track", { deckId, path });
    });
  }

  async function pickTrack(deckId: number) {
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
  }

  async function loadSample(deckId: number) {
    const samplePath = await invoke<string | null>("sample_track_path");
    if (!samplePath) {
      setError("Sample track not found. Use Load file instead.");
      return;
    }
    await loadTrack(deckId, samplePath);
  }

  async function playDeck(deckId: number) {
    await runAction(async () => {
      await invoke("play_deck", { deckId });
    });
  }

  async function pauseDeck(deckId: number) {
    await runAction(async () => {
      await invoke("pause_deck", { deckId });
    });
  }

  return (
    <div className="flex min-h-screen flex-col gap-5 bg-zinc-950 bg-[radial-gradient(circle_at_top_left,rgba(56,189,248,0.08),transparent_35%),radial-gradient(circle_at_top_right,rgba(168,85,247,0.08),transparent_35%)] p-6">
      <header className="flex flex-wrap items-start justify-between gap-4">
        <div>
          <h1 className="text-2xl font-semibold tracking-tight">Rust DJ Engine</h1>
          <p className="mt-1 text-sm text-zinc-400">
            Tauri prototype — two decks, real audio output
          </p>
        </div>

        <div className="flex flex-col items-end gap-2">
          <button
            type="button"
            className={
              status?.running
                ? `${buttonBase} border-red-500/45 bg-red-500/15 hover:bg-red-500/25`
                : `${buttonBase} border-emerald-500/45 bg-emerald-500/15 hover:bg-emerald-500/25`
            }
            disabled={busy}
            onClick={toggleEngine}
          >
            {status?.running ? "Stop engine" : "Start engine"}
          </button>

          <div className="flex items-center gap-3">
            <span className={statusPill(Boolean(status?.running))}>
              {status?.running ? "Running" : "Stopped"}
            </span>
            {status?.running && (
              <span className="text-xs text-zinc-400">
                {status.backend} · {status.sample_rate} Hz
              </span>
            )}
          </div>
        </div>
      </header>

      {error && (
        <div className="rounded-xl border border-red-500/35 bg-red-500/15 px-4 py-3 text-sm text-red-200">
          {error}
        </div>
      )}

      <main className="grid flex-1 gap-4 md:grid-cols-2">
        {(status?.decks ?? DECK_LABELS.map((_, id) => ({ id, track: null, playing: false }))).map(
          (deck, index) => (
            <section
              key={deck.id}
              className="flex min-h-64 flex-col gap-4 rounded-2xl border border-white/8 bg-white/3 p-5"
            >
              <div className="flex items-center justify-between">
                <h2 className="text-lg font-semibold">{DECK_LABELS[index]}</h2>
                <span className={statusPill(deck.playing)}>
                  {deck.playing ? "Playing" : "Idle"}
                </span>
              </div>

              <p
                className="truncate rounded-lg border border-dashed border-white/12 bg-black/35 px-4 py-3 text-sm text-slate-300"
                title={deck.track ?? undefined}
              >
                {fileName(deck.track)}
              </p>

              <div className="flex flex-wrap gap-2">
                <button
                  type="button"
                  className={`${buttonBase} border-white/12 bg-white/6 hover:border-white/20 hover:bg-white/10`}
                  disabled={busy || !status?.running}
                  onClick={() => pickTrack(deck.id)}
                >
                  Load file
                </button>
                <button
                  type="button"
                  className={`${buttonBase} border-sky-500/35 bg-sky-500/12 hover:bg-sky-500/20`}
                  disabled={busy || !status?.running}
                  onClick={() => loadSample(deck.id)}
                >
                  Load sample
                </button>
              </div>

              <div className="flex flex-wrap gap-2">
                <button
                  type="button"
                  className={`${buttonBase} border-violet-500/45 bg-violet-500/15 hover:bg-violet-500/25`}
                  disabled={busy || !status?.running || !deck.track}
                  onClick={() => playDeck(deck.id)}
                >
                  Play
                </button>
                <button
                  type="button"
                  className={`${buttonBase} border-white/12 bg-white/6 hover:border-white/20 hover:bg-white/10`}
                  disabled={busy || !status?.running || !deck.playing}
                  onClick={() => pauseDeck(deck.id)}
                >
                  Pause
                </button>
              </div>
            </section>
          ),
        )}
      </main>

      <footer className="text-sm text-slate-500">
        Start the engine, load a track on one or both decks, then hit Play.
      </footer>
    </div>
  );
}

export default App;
