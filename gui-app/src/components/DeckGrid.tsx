import { DECK_ACCENTS, DECK_LABELS } from "../lib/ui";
import type { DeckStatus } from "../types";
import { DeckPanel } from "./DeckPanel";

interface DeckGridProps {
  decks: DeckStatus[];
  engineRunning: boolean;
  busy: boolean;
  onPickTrack: (deckId: number) => void;
  onTogglePlayback: (deckId: number, playing: boolean) => void;
}

function defaultDecks(): DeckStatus[] {
  return DECK_LABELS.map((_, id) => ({ id, track: null, playing: false }));
}

export function DeckGrid({
  decks,
  engineRunning,
  busy,
  onPickTrack,
  onTogglePlayback,
}: DeckGridProps) {
  const deckList = decks.length > 0 ? decks : defaultDecks();
  const accents = [DECK_ACCENTS.a, DECK_ACCENTS.b] as const;

  return (
    <section className="grid max-h-[min(40vh,22rem)] min-h-0 shrink-0 grid-cols-2 overflow-y-auto border-b border-white/8 md:max-h-[min(45vh,28rem)] md:grid-cols-[1fr_auto_1fr]">
      <DeckPanel
        accent={accents[0]}
        deck={deckList[0]}
        engineRunning={engineRunning}
        busy={busy}
        onPickTrack={() => onPickTrack(deckList[0].id)}
        onTogglePlayback={() =>
          onTogglePlayback(deckList[0].id, deckList[0].playing)
        }
      />

      <div
        aria-hidden
        className="hidden flex-col items-center justify-end gap-2 border-x border-white/6 bg-zinc-900/50 px-2 py-3 md:flex"
      >
        <span className="text-[10px] font-semibold uppercase tracking-widest text-zinc-600">
          Mixer
        </span>
        <div className="flex flex-1 flex-col items-center justify-center gap-2">
          <div className="h-16 w-1.5 rounded-full bg-zinc-800 lg:h-20" />
          <div className="h-16 w-1.5 rounded-full bg-zinc-800 lg:h-20" />
        </div>
        <div className="h-1 w-12 rounded-full bg-zinc-800" />
      </div>

      <DeckPanel
        accent={accents[1]}
        deck={deckList[1]}
        engineRunning={engineRunning}
        busy={busy}
        onPickTrack={() => onPickTrack(deckList[1].id)}
        onTogglePlayback={() =>
          onTogglePlayback(deckList[1].id, deckList[1].playing)
        }
      />
    </section>
  );
}
