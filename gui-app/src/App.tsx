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
    <div className="flex h-screen flex-col overflow-hidden bg-zinc-950 text-zinc-100">
      <AppHeader status={status} busy={busy} onToggleEngine={toggleEngine} />

      {error && (
        <div className="shrink-0 px-4 pt-3">
          <MessageBanner message={error} variant="error" />
        </div>
      )}

      <DeckGrid
        decks={status?.decks ?? []}
        engineRunning={engineRunning}
        busy={busy}
        onPickTrack={pickTrack}
        onLoadSample={loadSample}
        onPlay={playDeck}
        onPause={pauseDeck}
      />

      <LibraryPanel
        engineRunning={engineRunning}
        engineBusy={busy}
        onLoadToDeck={loadLibraryTrackToDeck}
      />
    </div>
  );
}

export default App;
