import {
  deckDisplayTitle,
  formatOptional,
} from "../lib/format";
import type { DeckStatus } from "../types";

interface DeckTrackInfoProps {
  deck: DeckStatus;
}

export function DeckTrackInfo({ deck }: DeckTrackInfoProps) {
  const hasTrack = Boolean(deck.track);

  return (
    <div className="flex min-w-0 shrink-0 flex-col gap-1">
      <div className="flex min-w-0 items-start justify-between gap-2">
        <div className="min-w-0 flex-1">
          <p
            className="select-text truncate text-sm font-semibold leading-tight text-zinc-100"
            title={deckDisplayTitle(deck)}
          >
            {hasTrack ? deckDisplayTitle(deck) : "No track loaded"}
          </p>
          <p
            className="select-text truncate text-[11px] text-zinc-500"
            title={deck.artist ?? undefined}
          >
            {hasTrack ? formatOptional(deck.artist) : "Drop or load a track"}
          </p>
        </div>
        <span className="shrink-0 text-[11px] font-medium tabular-nums text-zinc-400">
          {formatOptional(deck.key)}
        </span>
      </div>
    </div>
  );
}
