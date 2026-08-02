import { DeckMixer } from "@/components/mixer/DeckMixer";
import { DeckPanel } from "./DeckPanel";

/** Fixed-height deck/mixer/deck row (410px). Waveform lives in MixerPage above this. */
export function DeckGrid() {
  return (
    <section className="grid h-full min-h-0 grid-cols-[minmax(0,1fr)_auto_minmax(0,1fr)] overflow-hidden">
      <div className="col-start-1 min-h-0 min-w-0 overflow-hidden">
        <DeckPanel deckId={0} accentKey="a" />
      </div>

      <div className="col-start-2 h-full min-h-0 shrink-0">
        <DeckMixer />
      </div>

      <div className="col-start-3 min-h-0 min-w-0 overflow-hidden">
        <DeckPanel deckId={1} accentKey="b" />
      </div>
    </section>
  );
}
