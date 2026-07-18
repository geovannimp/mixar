import { useEffect, useState } from "react";
import { ResizableHandle, ResizablePanel, ResizablePanelGroup } from "@/components/ui/resizable";
import { DeckGrid } from "@/components/DeckGrid";
import { DualDeckWaveform } from "@/components/DualDeckWaveform";
import { LibraryPanel } from "@/components/LibraryPanel";
import { useDeckHotkeys } from "@/hooks/useDeckHotkeys";
import { engineActions, useDeckControls } from "@/hooks/useEngine";

const WAVEFORM_MIN_HEIGHT = "70px";
const WAVEFORM_DEFAULT_HEIGHT = "112px";
const WAVEFORM_MAX_HEIGHT = "400px";
const DECK_ROW_HEIGHT = "410px";

export function MixerPage() {
  const [focusedDeckId, setFocusedDeckId] = useState(0);
  const focusedDeck = useDeckControls(focusedDeckId);
  const { ensureEngineRunning, triggerHotCue, beatJumpDeck, beginLoopRoll, endLoopRoll } =
    engineActions;

  useEffect(() => {
    void ensureEngineRunning();
  }, [ensureEngineRunning]);

  useDeckHotkeys({
    focusedDeckId,
    padMode: focusedDeck.pad_mode,
    onTriggerHotCue: triggerHotCue,
    onBeatJump: beatJumpDeck,
    onBeginLoopRoll: beginLoopRoll,
    onEndLoopRoll: endLoopRoll,
  });

  return (
    <div className="flex min-h-0 flex-1 select-none flex-col">
      <ResizablePanelGroup id="mixer-layout" orientation="vertical" className="min-h-0 flex-1">
        <ResizablePanel
          id="waveforms"
          defaultSize={WAVEFORM_DEFAULT_HEIGHT}
          minSize={WAVEFORM_MIN_HEIGHT}
          maxSize={WAVEFORM_MAX_HEIGHT}
          className="min-h-0 overflow-hidden"
        >
          <DualDeckWaveform />
        </ResizablePanel>

        <ResizableHandle withHandle className="bg-white/8 hover:bg-emerald-500/25" />

        <ResizablePanel
          id="decks"
          defaultSize={DECK_ROW_HEIGHT}
          minSize={DECK_ROW_HEIGHT}
          maxSize={DECK_ROW_HEIGHT}
          disabled
          className="min-h-0 overflow-hidden"
        >
          <DeckGrid focusedDeckId={focusedDeckId} onFocusDeck={setFocusedDeckId} />
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
    </div>
  );
}
