import { DECK_LABELS, buttonCompact } from "../lib/ui";
import { fileName } from "../lib/format";
import type { CollectionSummary, TrackSummary } from "../types";

interface TrackListProps {
  tracks: TrackSummary[];
  selectedCollection: CollectionSummary | undefined;
  engineRunning: boolean;
  busy: boolean;
  onLoadToDeck: (deckId: number, trackId: string) => void;
}

export function TrackList({
  tracks,
  selectedCollection,
  engineRunning,
  busy,
  onLoadToDeck,
}: TrackListProps) {
  if (tracks.length === 0) {
    return (
      <p className="rounded-lg border border-dashed border-white/12 px-3 py-4 text-sm text-zinc-500">
        {selectedCollection
          ? "No file tracks in this collection."
          : "Select a collection to browse tracks."}
      </p>
    );
  }

  return (
    <ul className="flex max-h-72 flex-col gap-2 overflow-y-auto">
      {tracks.map((track) => (
        <li
          key={track.id}
          className="flex items-center justify-between gap-3 rounded-lg border border-white/10 bg-black/20 px-3 py-2"
        >
          <div className="min-w-0">
            <p className="truncate text-sm font-medium">{track.display_name}</p>
            <p className="truncate text-xs text-zinc-500" title={track.path}>
              {fileName(track.path)}
            </p>
          </div>
          <div className="flex shrink-0 gap-1">
            {DECK_LABELS.map((label, deckId) => (
              <button
                key={label}
                type="button"
                className={`${buttonCompact} border-violet-500/35 bg-violet-500/10 hover:bg-violet-500/20`}
                disabled={busy || !engineRunning}
                title={`Load onto ${label}`}
                onClick={() => onLoadToDeck(deckId, track.id)}
              >
                {label.replace("Deck ", "")}
              </button>
            ))}
          </div>
        </li>
      ))}
    </ul>
  );
}
