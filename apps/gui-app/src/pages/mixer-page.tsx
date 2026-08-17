import { useEffect } from "react";
import { ResizableHandle, ResizablePanel, ResizablePanelGroup } from "@/components/ui/resizable";
import { DeckGrid } from "@/components/deck/deck-grid";
import { DualDeckWaveform } from "@/components/waveform/dual-deck-waveform";
import { LibraryPanel } from "@/components/library/library-panel";
import { TrackDragProvider } from "@/components/dnd/track-drag-provider";
import { engineActions } from "@/stores/engine-store";

const WAVEFORM_MIN_HEIGHT = "70px";
const WAVEFORM_DEFAULT_HEIGHT = "112px";
const WAVEFORM_MAX_HEIGHT = "400px";
const DECK_ROW_HEIGHT = "410px";

export function MixerPage() {
  const { ensureEngineRunning } = engineActions;

  useEffect(() => {
    void ensureEngineRunning();
  }, [ensureEngineRunning]);

  return (
    <div className="flex min-h-0 flex-1 select-none flex-col">
      <ResizablePanelGroup id="mixer-layout" orientation="vertical" className="min-h-0 flex-1">
        <ResizablePanel
          id="waveforms"
          defaultSize={WAVEFORM_DEFAULT_HEIGHT}
          minSize={WAVEFORM_MIN_HEIGHT}
          maxSize={WAVEFORM_MAX_HEIGHT}
          groupResizeBehavior="preserve-pixel-size"
          className="min-h-0 overflow-hidden"
        >
          <DualDeckWaveform />
        </ResizablePanel>

        <ResizableHandle withHandle className="bg-white/8 hover:bg-emerald-500/25" />

        {/* Keep waveforms outside TrackDragProvider so drag state updates don't re-render lanes. */}
        <ResizablePanel id="decks-and-library" defaultSize="60" minSize="40" className="min-h-0">
          <TrackDragProvider>
            <ResizablePanelGroup
              id="mixer-decks-library"
              orientation="vertical"
              className="h-full min-h-0"
            >
              <ResizablePanel
                id="decks"
                defaultSize={DECK_ROW_HEIGHT}
                minSize={DECK_ROW_HEIGHT}
                maxSize={DECK_ROW_HEIGHT}
                disabled
                className="min-h-0 overflow-hidden"
              >
                <DeckGrid />
              </ResizablePanel>

              <ResizableHandle withHandle className="bg-white/8 hover:bg-emerald-500/25" />

              <ResizablePanel
                id="library"
                defaultSize="60"
                minSize="30"
                className="min-h-0 overflow-hidden"
              >
                <LibraryPanel />
              </ResizablePanel>
            </ResizablePanelGroup>
          </TrackDragProvider>
        </ResizablePanel>
      </ResizablePanelGroup>
    </div>
  );
}
