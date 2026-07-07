import { DECK_ACCENTS, DECK_LABELS } from "../lib/ui";
import {
  formatBpm,
  formatDuration,
  formatOptional,
} from "../lib/format";
import type { CollectionSummary, TrackSummary } from "../types";

interface TrackListProps {
  tracks: TrackSummary[];
  selectedCollection: CollectionSummary | undefined;
  engineRunning: boolean;
  busy: boolean;
  analyzingTrackId: string | null;
  onLoadToDeck: (deckId: number, trackId: string) => void;
  onAnalyze: (trackId: string) => void;
}

const deckAccentKeys = ["a", "b"] as const;

function trackTitle(track: TrackSummary): string {
  return track.title?.trim() || track.display_name;
}

export function TrackList({
  tracks,
  selectedCollection,
  engineRunning,
  busy,
  analyzingTrackId,
  onLoadToDeck,
  onAnalyze,
}: TrackListProps) {
  if (tracks.length === 0) {
    return (
      <p className="rounded border border-dashed border-white/10 px-4 py-8 text-center text-sm text-zinc-500">
        {selectedCollection
          ? "No file tracks in this collection."
          : "Select a collection to browse tracks."}
      </p>
    );
  }

  return (
    <table className="w-full min-w-[40rem] border-collapse text-sm">
      <thead className="sticky top-0 z-10 bg-zinc-900/95 text-left text-[10px] font-semibold uppercase tracking-widest text-zinc-500">
        <tr className="border-b border-white/8">
          <th className="px-2 py-2 font-semibold">Title</th>
          <th className="hidden px-2 py-2 font-semibold sm:table-cell">Artist</th>
          <th className="px-2 py-2 font-semibold">BPM</th>
          <th className="px-2 py-2 font-semibold">Key</th>
          <th className="px-2 py-2 font-semibold">Length</th>
          <th className="hidden px-2 py-2 font-semibold lg:table-cell">Genre</th>
          <th className="px-2 py-2 text-right font-semibold">Analyze</th>
          <th className="px-2 py-2 text-right font-semibold">Load</th>
        </tr>
      </thead>
      <tbody>
        {tracks.map((track) => (
          <tr
            key={track.id}
            className="border-b border-white/5 transition hover:bg-white/3"
          >
            <td className="max-w-[10rem] truncate px-2 py-1.5 font-medium sm:max-w-xs">
              {trackTitle(track)}
            </td>
            <td className="hidden max-w-[8rem] truncate px-2 py-1.5 text-zinc-400 sm:table-cell sm:max-w-xs">
              {formatOptional(track.artist)}
            </td>
            <td className="whitespace-nowrap px-2 py-1.5 tabular-nums text-zinc-300">
              {formatBpm(track.bpm)}
            </td>
            <td className="whitespace-nowrap px-2 py-1.5 text-zinc-300">
              {formatOptional(track.key)}
            </td>
            <td className="whitespace-nowrap px-2 py-1.5 tabular-nums text-zinc-400">
              {formatDuration(track.duration_secs)}
            </td>
            <td className="hidden max-w-[6rem] truncate px-2 py-1.5 text-zinc-500 lg:table-cell">
              {formatOptional(track.genre)}
            </td>
            <td className="px-2 py-1.5">
              <div className="flex justify-end">
                <button
                  type="button"
                  className="rounded border border-amber-500/35 bg-amber-500/10 px-2 py-0.5 text-xs font-semibold transition hover:bg-amber-500/20 disabled:cursor-not-allowed disabled:opacity-45"
                  disabled={busy || analyzingTrackId === track.id}
                  title="Analyze track (BPM, key)"
                  onClick={() => onAnalyze(track.id)}
                >
                  {analyzingTrackId === track.id ? "…" : "An"}
                </button>
              </div>
            </td>
            <td className="px-2 py-1.5">
              <div className="flex justify-end gap-1">
                {DECK_LABELS.map((label, deckId) => {
                  const accent = DECK_ACCENTS[deckAccentKeys[deckId]];
                  return (
                    <button
                      key={label}
                      type="button"
                      className={`rounded border px-2 py-0.5 text-xs font-semibold transition disabled:cursor-not-allowed disabled:opacity-45 ${accent.button}`}
                      disabled={busy || !engineRunning}
                      title={`Load onto ${label}`}
                      onClick={() => onLoadToDeck(deckId, track.id)}
                    >
                      {label.replace("Deck ", "")}
                    </button>
                  );
                })}
              </div>
            </td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}
