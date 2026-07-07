import { DECK_LABELS } from "../lib/ui";
import type { DeckStatus } from "../types";
import { DeckPanel } from "./DeckPanel";

interface DeckGridProps {
  decks: DeckStatus[];
  engineRunning: boolean;
  busy: boolean;
  onPickTrack: (deckId: number) => void;
  onLoadSample: (deckId: number) => void;
  onPlay: (deckId: number) => void;
  onPause: (deckId: number) => void;
}

function defaultDecks(): DeckStatus[] {
  return DECK_LABELS.map((_, id) => ({ id, track: null, playing: false }));
}

export function DeckGrid({
  decks,
  engineRunning,
  busy,
  onPickTrack,
  onLoadSample,
  onPlay,
  onPause,
}: DeckGridProps) {
  const deckList = decks.length > 0 ? decks : defaultDecks();

  return (
    <main className="grid flex-1 gap-4 md:grid-cols-2">
      {deckList.map((deck, index) => (
        <DeckPanel
          key={deck.id}
          label={DECK_LABELS[index]}
          deck={deck}
          engineRunning={engineRunning}
          busy={busy}
          onPickTrack={() => onPickTrack(deck.id)}
          onLoadSample={() => onLoadSample(deck.id)}
          onPlay={() => onPlay(deck.id)}
          onPause={() => onPause(deck.id)}
        />
      ))}
    </main>
  );
}
