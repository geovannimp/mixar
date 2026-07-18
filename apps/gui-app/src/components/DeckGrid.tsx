import { DeckMixer } from "./DeckMixer";
import { DeckPanel } from "./DeckPanel";

interface DeckGridProps {
  focusedDeckId: number;
  onFocusDeck: (deckId: number) => void;
}

/** Fixed-height deck/mixer/deck row (410px). Waveform lives in MixerPage above this. */
export function DeckGrid({ focusedDeckId, onFocusDeck }: DeckGridProps) {
  return (
    <section className="grid h-full min-h-0 grid-cols-[minmax(0,1fr)_auto_minmax(0,1fr)] overflow-hidden">
      <div className="col-start-1 min-h-0 min-w-0 overflow-hidden">
        <DeckPanel
          deckId={0}
          accentKey="a"
          focused={focusedDeckId === 0}
          onFocus={() => onFocusDeck(0)}
        />
      </div>

      <div className="col-start-2 h-full min-h-0 shrink-0">
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
    </section>
  );
}
