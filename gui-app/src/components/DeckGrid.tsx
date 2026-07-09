import { DeckMixer } from "./DeckMixer";
import { DualDeckWaveform } from "./DualDeckWaveform";
import {
  ResizableHandle,
  ResizablePanel,
  ResizablePanelGroup,
} from "@/components/ui/resizable";
import { DeckPanel } from "./DeckPanel";

/** Dual-lane scrolling waveform strip. */
const WAVEFORM_MIN_HEIGHT = "70px";
const WAVEFORM_DEFAULT_HEIGHT = "112px";
const WAVEFORM_MAX_HEIGHT = "400px";
const DECK_ROW_MIN_HEIGHT = "340px";
const DECK_ROW_DEFAULT_HEIGHT = "350px";

interface DeckGridProps {
  focusedDeckId: number;
  onFocusDeck: (deckId: number) => void;
}

export function DeckGrid({ focusedDeckId, onFocusDeck }: DeckGridProps) {
  return (
    <section className="flex h-full min-h-0 flex-col">
      <ResizablePanelGroup
        id="deck-waveform-split"
        orientation="vertical"
        className="min-h-0 flex-1"
      >
        <ResizablePanel
          id="waveforms"
          defaultSize={WAVEFORM_DEFAULT_HEIGHT}
          minSize={WAVEFORM_MIN_HEIGHT}
          maxSize={WAVEFORM_MAX_HEIGHT}
          className="min-h-0 overflow-hidden"
        >
          <DualDeckWaveform />
        </ResizablePanel>

        <ResizableHandle
          withHandle
          className="bg-white/8 hover:bg-emerald-500/25"
        />

        <ResizablePanel
          id="decks"
          defaultSize={DECK_ROW_DEFAULT_HEIGHT}
          minSize={DECK_ROW_MIN_HEIGHT}
          groupResizeBehavior="preserve-pixel-size"
          className="min-h-[340px] overflow-hidden"
        >
          <div className="grid h-full min-h-0 grid-cols-[minmax(0,1fr)_auto_minmax(0,1fr)]">
            <div className="col-start-1 min-h-0 min-w-0 overflow-hidden">
              <DeckPanel
                deckId={0}
                accentKey="a"
                focused={focusedDeckId === 0}
                onFocus={() => onFocusDeck(0)}
              />
            </div>

            <div className="col-start-2 min-h-0 shrink-0 overflow-hidden">
              <DeckMixer />
            </div>

            <div className="col-start-3 min-h-0 min-w-0 overflow-hidden">
              <DeckPanel
                deckId={1}
                accentKey="b"
                focused={focusedDeckId === 1}
                onFocus={() => onFocusDeck(1)}
              />
            </div>
          </div>
        </ResizablePanel>
      </ResizablePanelGroup>
    </section>
  );
}
