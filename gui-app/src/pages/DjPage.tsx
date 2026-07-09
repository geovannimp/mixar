import { useEffect } from "react";
import {
  ResizableHandle,
  ResizablePanel,
  ResizablePanelGroup,
} from "@/components/ui/resizable";
import { DeckGrid } from "../components/DeckGrid";
import { LibraryPanel } from "../components/LibraryPanel";
import { useEngine } from "../hooks/useEngine";
import type { TrackDragPayload } from "../lib/libraryTable";

export function DjPage() {
  const {
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
    seekDeck,
    unloadDeck,
    setDeckCuePoint,
    beginDeckCueHold,
    endDeckCueHold,
    setDeckQuantize,
    setDeckAutoLoop,
    setDeckLoopIn,
    setDeckLoopOut,
    exitDeckLoop,
    triggerHotCue,
    saveHotCue,
    deleteHotCue,
  } = useEngine();

  const engineRunning = Boolean(status?.running);

  useEffect(() => {
    void ensureEngineRunning();
  }, [ensureEngineRunning]);

  const toggleDeckPlayback = (deckId: number, playing: boolean) => {
    if (playing) {
      void pauseDeck(deckId);
      return;
    }
    void playDeck(deckId);
  };

  const loadDraggedTrack = (deckId: number, payload: TrackDragPayload) => {
    if (payload.source === "library" && payload.trackId) {
      void loadLibraryTrackToDeck(deckId, payload.trackId);
      return;
    }
    void loadPathToDeck(deckId, payload.path);
  };

  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <ResizablePanelGroup
        id="dj-layout"
        orientation="vertical"
        className="min-h-0 flex-1"
      >
        <ResizablePanel
          id="decks"
          defaultSize="480px"
          minSize="420px"
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
            onSpeedChange={setDeckSpeed}
            onDropTrack={loadDraggedTrack}
            onSeek={seekDeck}
            onSetCuePoint={setDeckCuePoint}
            onBeginCueHold={beginDeckCueHold}
            onEndCueHold={endDeckCueHold}
            onTriggerHotCue={triggerHotCue}
            onSaveHotCue={saveHotCue}
            onDeleteHotCue={deleteHotCue}
            onAutoLoop={setDeckAutoLoop}
            onLoopIn={setDeckLoopIn}
            onLoopOut={setDeckLoopOut}
            onExitLoop={exitDeckLoop}
            onToggleQuantize={setDeckQuantize}
            onUnload={unloadDeck}
            crossfader={status?.crossfader ?? 0.5}
            onCrossfaderChange={setCrossfader}
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
            onLoadPathToDeck={loadPathToDeck}
          />
        </ResizablePanel>
      </ResizablePanelGroup>
    </div>
  );
}
