import { useState } from "react";
import { Disc3 } from "lucide-react";
import { deckDisplayTitle, formatOptional } from "@/lib/format";
import { formatDeckKey, getKeyDisplayMode, setKeyDisplayMode } from "@/lib/keyFormat";
import { useTrackArtwork } from "@/hooks/library/useTrackArtwork";
import type { DeckStatus, KeyDisplayMode } from "@/types";

interface DeckTrackInfoProps {
  deck: DeckStatus;
}

export function DeckTrackInfo({ deck }: DeckTrackInfoProps) {
  const hasTrack = Boolean(deck.track);
  const [keyMode, setKeyMode] = useState<KeyDisplayMode>(getKeyDisplayMode);
  const artwork = useTrackArtwork(deck.track_id, deck.track);
  const displayKey = formatDeckKey(deck.key, keyMode);

  const cycleKeyMode = () => {
    const next: KeyDisplayMode = keyMode === "musical" ? "camelot" : "musical";
    setKeyDisplayMode(next);
    setKeyMode(next);
  };

  return (
    <div className="flex min-w-0 shrink-0 gap-2">
      <div
        className="flex size-11 shrink-0 items-center justify-center overflow-hidden rounded-md border border-white/10 bg-zinc-950/80 sm:size-12"
        aria-hidden={!hasTrack}
      >
        {artwork ? (
          <img src={artwork} alt="" className="size-full object-cover" />
        ) : (
          <Disc3 className="size-5 text-zinc-600" />
        )}
      </div>

      <div className="flex min-w-0 flex-1 flex-col gap-1">
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
          <button
            type="button"
            className="shrink-0 text-[11px] font-medium tabular-nums text-zinc-400 hover:text-zinc-200"
            title={`Key display: ${keyMode} — click to toggle`}
            disabled={!hasTrack}
            onClick={cycleKeyMode}
          >
            {displayKey}
          </button>
        </div>
      </div>
    </div>
  );
}
