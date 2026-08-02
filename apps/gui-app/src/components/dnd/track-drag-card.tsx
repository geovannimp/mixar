import { formatBpm, formatOptional } from "@/lib/format";
import { rowArtist, rowBpmValue, rowMusicalKey, rowTitle } from "@/lib/library-table";
import type { LibraryTableRow } from "@/types";

interface TrackDragCardProps {
  row: LibraryTableRow;
}

function coverGradient(seed: string): string {
  let hash = 0;
  for (let index = 0; index < seed.length; index += 1) {
    hash = seed.charCodeAt(index) + ((hash << 5) - hash);
  }
  const hue = Math.abs(hash) % 360;
  return `linear-gradient(135deg, hsl(${hue} 55% 34%) 0%, hsl(${(hue + 48) % 360} 48% 22%) 100%)`;
}

function coverInitial(row: LibraryTableRow): string {
  const title = rowTitle(row).trim();
  if (!title) {
    return "?";
  }
  return title.charAt(0).toUpperCase();
}

export function TrackDragCard({ row }: TrackDragCardProps) {
  const title = rowTitle(row);
  const artist = formatOptional(rowArtist(row));
  const bpm = formatBpm(rowBpmValue(row));
  const key = formatOptional(rowMusicalKey(row));

  return (
    <div className="flex w-48 items-center gap-2 rounded-lg border border-white/12 bg-zinc-950/95 p-1.5 shadow-xl shadow-black/50 backdrop-blur-sm">
      <div
        className="flex size-9 shrink-0 items-center justify-center overflow-hidden rounded-md border border-white/10 text-xs font-semibold text-white/90"
        style={{ background: coverGradient(title + artist) }}
      >
        <span className="sr-only">{title}</span>
        <span aria-hidden>{coverInitial(row)}</span>
      </div>
      <div className="min-w-0 flex-1">
        <p className="truncate text-xs font-semibold text-zinc-100">{title}</p>
        <p className="truncate text-[10px] text-zinc-400">{artist}</p>
        <div className="mt-1 flex gap-1">
          <span className="rounded border border-white/10 bg-white/5 px-1.5 py-px text-[9px] font-medium text-zinc-300">
            {bpm}
          </span>
          <span className="rounded border border-white/10 bg-white/5 px-1.5 py-px text-[9px] font-medium text-zinc-300">
            {key}
          </span>
        </div>
      </div>
    </div>
  );
}
