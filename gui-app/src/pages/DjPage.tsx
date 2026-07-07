import { useDefaultLayout } from "react-resizable-panels";
import {
  ResizableHandle,
  ResizablePanel,
  ResizablePanelGroup,
} from "@/components/ui/resizable";
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
    setDeckVolume,
    setDeckEq,
  } = useEngine();

  const engineRunning = Boolean(status?.running);

  const djLayout = useDefaultLayout({
    id: "dj-layout-v2",
    panelIds: ["decks", "library"],
  });

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

      <ResizablePanelGroup
        id="dj-layout-v2"
        orientation="vertical"
        className="min-h-0 flex-1"
        defaultLayout={djLayout.defaultLayout}
        onLayoutChanged={djLayout.onLayoutChanged}
      >
        <ResizablePanel
          id="decks"
          defaultSize="40"
          minSize="25"
          className="min-h-0 overflow-hidden"
        >
          <DeckGrid
            decks={status?.decks ?? []}
            engineRunning={engineRunning}
            busy={busy}
            onPickTrack={pickTrack}
            onTogglePlayback={toggleDeckPlayback}
            onVolumeChange={setDeckVolume}
            onEqChange={setDeckEq}
          />
        </ResizablePanel>

        <ResizableHandle
          withHandle
          className="bg-white/8 hover:bg-emerald-500/25"
        />

        <ResizablePanel
          id="library"
          defaultSize="60"
          minSize="30"
          className="min-h-0 overflow-hidden"
        >
          <LibraryPanel
            engineRunning={engineRunning}
            engineBusy={busy}
            onLoadToDeck={loadLibraryTrackToDeck}
          />
        </ResizablePanel>
      </ResizablePanelGroup>
    </div>
  );
}
