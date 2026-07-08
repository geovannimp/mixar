import { DECK_ACCENTS, DECK_LABELS } from "../lib/ui";
import type { FsEntry } from "../types";

interface DriveFileListProps {
  audioFiles: FsEntry[];
  currentPath: string | null;
  engineRunning: boolean;
  busy: boolean;
  onLoadToDeck: (deckId: number, path: string) => void;
}

const deckAccentKeys = ["a", "b"] as const;

export function DriveFileList({
  audioFiles,
  currentPath,
  engineRunning,
  busy,
  onLoadToDeck,
}: DriveFileListProps) {
  if (!currentPath) {
    return (
      <p className="rounded border border-dashed border-white/10 px-4 py-8 text-center text-sm text-zinc-500">
        Select a drive or folder to browse audio files.
      </p>
    );
  }

  if (audioFiles.length === 0) {
    return (
      <p className="rounded border border-dashed border-white/10 px-4 py-8 text-center text-sm text-zinc-500">
        No audio files in this folder.
      </p>
    );
  }

  return (
    <table className="w-full min-w-[28rem] border-collapse text-sm">
      <thead className="sticky top-0 z-10 bg-zinc-900/95 text-left text-[10px] font-semibold uppercase tracking-widest text-zinc-500">
        <tr className="border-b border-white/8">
          <th className="px-2 py-2 font-semibold">File</th>
          <th className="px-2 py-2 text-right font-semibold">Load</th>
        </tr>
      </thead>
      <tbody>
        {audioFiles.map((file) => (
          <tr
            key={file.path}
            className="border-b border-white/5 transition hover:bg-white/3"
          >
            <td className="max-w-xs truncate px-2 py-1.5 font-medium">
              {file.name}
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
                      onClick={() => onLoadToDeck(deckId, file.path)}
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
