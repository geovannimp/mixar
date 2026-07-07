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

interface CollectionSummary {
  id: string;
  name: string;
  kind: string;
  path: string | null;
  track_count: number;
}

interface TrackSummary {
  id: string;
  display_name: string;
  artist: string | null;
  title: string | null;
  path: string;
}

interface ScanReport {
  added: number;
  updated: number;
  skipped: number;
  failed: number;
  errors: string[];
}

interface AddFolderCollectionResult {
  collection: CollectionSummary;
  scan: ScanReport;
}

const DECK_LABELS = ["Deck A", "Deck B"] as const;

const buttonBase =
  "rounded-lg border px-4 py-2.5 text-sm font-medium transition disabled:cursor-not-allowed disabled:opacity-45";

const buttonCompact =
  "rounded-md border px-2.5 py-1 text-xs font-medium transition disabled:cursor-not-allowed disabled:opacity-45";

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
  const [collections, setCollections] = useState<CollectionSummary[]>([]);
  const [selectedCollectionId, setSelectedCollectionId] = useState<string | null>(null);
  const [tracks, setTracks] = useState<TrackSummary[]>([]);
  const [scanMessage, setScanMessage] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const refreshStatus = useCallback(async () => {
    const next = await invoke<EngineStatus>("get_status");
    setStatus(next);
  }, []);

  const refreshCollections = useCallback(async () => {
    const next = await invoke<CollectionSummary[]>("list_collections");
    setCollections(next);
    if (next.length === 0) {
      setSelectedCollectionId(null);
      setTracks([]);
      return;
    }
    setSelectedCollectionId((current) => {
      if (current && next.some((collection) => collection.id === current)) {
        return current;
      }
      return next[0]?.id ?? null;
    });
  }, []);

  const refreshTracks = useCallback(async (collectionId: string) => {
    const next = await invoke<TrackSummary[]>("list_collection_tracks", { collectionId });
    setTracks(next);
  }, []);

  useEffect(() => {
    refreshStatus().catch((err: unknown) => {
      setError(String(err));
    });
    refreshCollections().catch((err: unknown) => {
      setError(String(err));
    });
  }, [refreshStatus, refreshCollections]);

  useEffect(() => {
    if (!selectedCollectionId) {
      setTracks([]);
      return;
    }
    refreshTracks(selectedCollectionId).catch((err: unknown) => {
      setError(String(err));
    });
  }, [selectedCollectionId, refreshTracks]);

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

  async function addFolderCollection() {
    const selected = await open({
      directory: true,
      multiple: false,
    });
    if (typeof selected !== "string") {
      return;
    }

    setBusy(true);
    setError(null);
    setScanMessage(null);
    try {
      const result = await invoke<AddFolderCollectionResult>("add_folder_collection", {
        folderPath: selected,
      });
      setSelectedCollectionId(result.collection.id);
      await refreshCollections();
      await refreshTracks(result.collection.id);
      setScanMessage(
        `Imported ${result.scan.added} tracks, updated ${result.scan.updated}, skipped ${result.scan.skipped}.`,
      );
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  }

  async function loadLibraryTrackToDeck(deckId: number, trackId: string) {
    await runAction(async () => {
      await invoke("load_library_track_to_deck", { deckId, trackId });
    });
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

  const selectedCollection = collections.find(
    (collection) => collection.id === selectedCollectionId,
  );

  return (
    <div className="flex min-h-screen flex-col gap-5 bg-zinc-950 bg-[radial-gradient(circle_at_top_left,rgba(56,189,248,0.08),transparent_35%),radial-gradient(circle_at_top_right,rgba(168,85,247,0.08),transparent_35%)] p-6">
      <header className="flex flex-wrap items-start justify-between gap-4">
        <div>
          <h1 className="text-2xl font-semibold tracking-tight">Rust DJ Engine</h1>
          <p className="mt-1 text-sm text-zinc-400">
            Tauri prototype — library collections and two-deck playback
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

      {scanMessage && (
        <div className="rounded-xl border border-emerald-500/35 bg-emerald-500/10 px-4 py-3 text-sm text-emerald-200">
          {scanMessage}
        </div>
      )}

      <section className="rounded-2xl border border-white/8 bg-white/3 p-5">
        <div className="flex flex-wrap items-center justify-between gap-3">
          <div>
            <h2 className="text-lg font-semibold">Library</h2>
            <p className="mt-1 text-sm text-zinc-400">
              Add a folder collection, then load tracks onto a deck.
            </p>
          </div>
          <button
            type="button"
            className={`${buttonBase} border-amber-500/35 bg-amber-500/12 hover:bg-amber-500/20`}
            disabled={busy}
            onClick={addFolderCollection}
          >
            Add folder collection
          </button>
        </div>

        <div className="mt-4 grid gap-4 lg:grid-cols-[minmax(220px,280px)_1fr]">
          <div className="flex flex-col gap-2">
            <p className="text-xs font-semibold uppercase tracking-wide text-zinc-500">
              Collections
            </p>
            {collections.length === 0 ? (
              <p className="rounded-lg border border-dashed border-white/12 px-3 py-4 text-sm text-zinc-500">
                No collections yet. Add a folder to scan audio files.
              </p>
            ) : (
              <ul className="flex max-h-72 flex-col gap-2 overflow-y-auto">
                {collections.map((collection) => {
                  const selected = collection.id === selectedCollectionId;
                  return (
                    <li key={collection.id}>
                      <button
                        type="button"
                        className={
                          selected
                            ? "w-full rounded-lg border border-sky-500/40 bg-sky-500/10 px-3 py-2 text-left"
                            : "w-full rounded-lg border border-white/10 bg-black/20 px-3 py-2 text-left hover:border-white/20 hover:bg-black/30"
                        }
                        onClick={() => setSelectedCollectionId(collection.id)}
                      >
                        <span className="block truncate font-medium">{collection.name}</span>
                        <span className="mt-1 block truncate text-xs text-zinc-400">
                          {collection.track_count} tracks
                          {collection.path ? ` · ${collection.path}` : ""}
                        </span>
                      </button>
                    </li>
                  );
                })}
              </ul>
            )}
          </div>

          <div className="flex min-h-48 flex-col gap-2">
            <p className="text-xs font-semibold uppercase tracking-wide text-zinc-500">
              {selectedCollection ? `${selectedCollection.name} tracks` : "Tracks"}
            </p>
            {tracks.length === 0 ? (
              <p className="rounded-lg border border-dashed border-white/12 px-3 py-4 text-sm text-zinc-500">
                {selectedCollection
                  ? "No file tracks in this collection."
                  : "Select a collection to browse tracks."}
              </p>
            ) : (
              <ul className="flex max-h-72 flex-col gap-2 overflow-y-auto">
                {tracks.map((track) => (
                  <li
                    key={track.id}
                    className="flex items-center justify-between gap-3 rounded-lg border border-white/10 bg-black/20 px-3 py-2"
                  >
                    <div className="min-w-0">
                      <p className="truncate text-sm font-medium">{track.display_name}</p>
                      <p className="truncate text-xs text-zinc-500" title={track.path}>
                        {fileName(track.path)}
                      </p>
                    </div>
                    <div className="flex shrink-0 gap-1">
                      {DECK_LABELS.map((label, deckId) => (
                        <button
                          key={label}
                          type="button"
                          className={`${buttonCompact} border-violet-500/35 bg-violet-500/10 hover:bg-violet-500/20`}
                          disabled={busy || !status?.running}
                          title={`Load onto ${label}`}
                          onClick={() => loadLibraryTrackToDeck(deckId, track.id)}
                        >
                          {label.replace("Deck ", "")}
                        </button>
                      ))}
                    </div>
                  </li>
                ))}
              </ul>
            )}
          </div>
        </div>
      </section>

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
        Add a folder collection, pick a track, load it to deck A or B, then hit Play.
      </footer>
    </div>
  );
}

export default App;
