import { DeckMixer } from "./DeckMixer";
import { DECK_ACCENTS, DECK_LABELS } from "../lib/ui";
import { DEFAULT_DECK_EQ, type DeckEq, type DeckStatus } from "../types";
import { DeckPanel } from "./DeckPanel";

interface DeckGridProps {
  decks: DeckStatus[];
  engineRunning: boolean;
  busy: boolean;
  onPickTrack: (deckId: number) => void;
  onTogglePlayback: (deckId: number, playing: boolean) => void;
  onVolumeChange: (deckId: number, volume: number) => void;
  onEqChange: (deckId: number, eq: DeckEq) => void;
}

function defaultDecks(): DeckStatus[] {
  return DECK_LABELS.map((_, id) => ({
    id,
    track: null,
    playing: false,
    volume: 1,
    eq: DEFAULT_DECK_EQ,
  }));
}

export function DeckGrid({
  decks,
  engineRunning,
  busy,
  onPickTrack,
  onTogglePlayback,
  onVolumeChange,
  onEqChange,
}: DeckGridProps) {
  const deckList = decks.length > 0 ? decks : defaultDecks();
  const accents = [DECK_ACCENTS.a, DECK_ACCENTS.b] as const;

  return (
    <section className="grid h-full min-h-0 grid-cols-2 grid-rows-1 md:grid-cols-[1fr_auto_1fr]">
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

      <div className="hidden h-full min-h-0 shrink-0 md:block">
        <DeckMixer
          decks={deckList}
          disabled={busy}
          onVolumeChange={onVolumeChange}
          onEqChange={onEqChange}
        />
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
