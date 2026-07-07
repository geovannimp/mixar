import { DeckGrid } from "../components/DeckGrid";
import { LibraryPanel } from "../components/LibraryPanel";
import { MessageBanner } from "../components/MessageBanner";
import { useEngine } from "../hooks/useEngine";

export function DjPage() {
  const {
    status,
    error,
    busy,
    loadLibraryTrackToDeck,
    pickTrack,
    playDeck,
    pauseDeck,
  } = useEngine();

  const engineRunning = Boolean(status?.running);

  const toggleDeckPlayback = (deckId: number, playing: boolean) => {
    if (playing) {
      void pauseDeck(deckId);
      return;
    }
    void playDeck(deckId);
  };

  return (
    <div className="flex min-h-0 flex-1 flex-col">
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
        onTogglePlayback={toggleDeckPlayback}
      />

      <LibraryPanel
        engineRunning={engineRunning}
        engineBusy={busy}
        onLoadToDeck={loadLibraryTrackToDeck}
      />
    </div>
  );
}
