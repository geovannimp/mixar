import { AppHeader } from "./components/AppHeader";
import { DeckGrid } from "./components/DeckGrid";
import { LibraryPanel } from "./components/LibraryPanel";
import { MessageBanner } from "./components/MessageBanner";
import { useEngine } from "./hooks/useEngine";

function App() {
  const {
    status,
    error,
    busy,
    toggleEngine,
    loadLibraryTrackToDeck,
    pickTrack,
    loadSample,
    playDeck,
    pauseDeck,
  } = useEngine();

  const engineRunning = Boolean(status?.running);

  return (
    <div className="flex min-h-screen flex-col gap-5 bg-zinc-950 bg-[radial-gradient(circle_at_top_left,rgba(56,189,248,0.08),transparent_35%),radial-gradient(circle_at_top_right,rgba(168,85,247,0.08),transparent_35%)] p-6">
      <AppHeader status={status} busy={busy} onToggleEngine={toggleEngine} />

      {error && <MessageBanner message={error} variant="error" />}

      <LibraryPanel
        engineRunning={engineRunning}
        engineBusy={busy}
        onLoadToDeck={loadLibraryTrackToDeck}
      />

      <DeckGrid
        decks={status?.decks ?? []}
        engineRunning={engineRunning}
        busy={busy}
        onPickTrack={pickTrack}
        onLoadSample={loadSample}
        onPlay={playDeck}
        onPause={pauseDeck}
      />

      <footer className="text-sm text-slate-500">
        Add a folder collection, pick a track, load it to deck A or B, then hit Play.
      </footer>
    </div>
  );
}

export default App;
