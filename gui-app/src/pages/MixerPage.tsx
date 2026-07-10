import { useEffect, useState } from "react";
import {
  ResizableHandle,
  ResizablePanel,
  ResizablePanelGroup,
} from "@/components/ui/resizable";
import { DeckGrid } from "../components/DeckGrid";
import { LibraryPanel } from "../components/LibraryPanel";
import { useDeckHotkeys } from "../hooks/useDeckHotkeys";
import { engineActions } from "../hooks/useEngine";

export function MixerPage() {
  const [focusedDeckId, setFocusedDeckId] = useState(0);
  const { ensureEngineRunning, triggerHotCue } = engineActions;

  useEffect(() => {
    void ensureEngineRunning();
  }, [ensureEngineRunning]);

  useDeckHotkeys({
    focusedDeckId,
    onTriggerHotCue: triggerHotCue,
  });

  return (
    <div className="flex min-h-0 flex-1 select-none flex-col">
      <ResizablePanelGroup
        id="mixer-layout"
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
            focusedDeckId={focusedDeckId}
            onFocusDeck={setFocusedDeckId}
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
          <LibraryPanel />
        </ResizablePanel>
      </ResizablePanelGroup>
    </div>
  );
}
