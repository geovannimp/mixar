import { ChevronLeft, Folder } from "lucide-react";
import { isAtVolumeRoot } from "../lib/driveVolumes";
import type { DirectoryListing, VolumeInfo } from "../types";
import { DrivePathBreadcrumbs } from "./DrivePathBreadcrumbs";
import { DriveVolumeList } from "./DriveSelector";

interface DriveBrowserProps {
  volumes: VolumeInfo[];
  selectedVolume: VolumeInfo | null;
  listing: DirectoryListing | null;
  busy: boolean;
  onSelectVolume: (path: string) => void;
  onOpenDirectory: (path: string) => void;
  onGoUp: () => void;
}

export function DriveBrowser({
  volumes,
  selectedVolume,
  listing,
  busy,
  onSelectVolume,
  onOpenDirectory,
  onGoUp,
}: DriveBrowserProps) {
  return (
    <div className="flex min-h-0 flex-1 flex-col gap-2">
      {!listing ? (
        <DriveVolumeList
          volumes={volumes}
          busy={busy}
          onSelectVolume={onSelectVolume}
        />
      ) : (
        <>
          <div className="flex shrink-0 items-center gap-1">
            {listing.parent &&
              !isAtVolumeRoot(listing.path, selectedVolume) && (
              <button
                type="button"
                className="flex h-6 w-6 items-center justify-center rounded border border-white/10 text-zinc-400 transition hover:bg-white/5 hover:text-zinc-200 disabled:cursor-not-allowed disabled:opacity-45"
                disabled={busy}
                title="Parent folder"
                aria-label="Parent folder"
                onClick={onGoUp}
              >
                <ChevronLeft className="size-3.5" aria-hidden />
              </button>
            )}
            <DrivePathBreadcrumbs
              listingPath={listing.path}
              selectedVolume={selectedVolume}
              busy={busy}
              onOpenDirectory={onOpenDirectory}
            />
          </div>

          {listing.directories.length === 0 ? (
            <p className="rounded border border-dashed border-white/10 px-3 py-4 text-sm text-zinc-500">
              No subfolders here.
            </p>
          ) : (
            <ul className="flex min-h-0 flex-1 flex-col gap-0.5 overflow-y-auto">
              {listing.directories.map((directory) => (
                <li key={directory.path}>
                  <button
                    type="button"
                    className="w-full rounded border-l-2 border-l-transparent px-3 py-2 text-left hover:bg-white/5"
                    disabled={busy}
                    onClick={() => onOpenDirectory(directory.path)}
                  >
                    <span className="flex items-center gap-2">
                      <Folder
                        className="size-4 shrink-0 text-zinc-500"
                        aria-hidden
                      />
                      <span className="truncate text-sm">{directory.name}</span>
                    </span>
                  </button>
                </li>
              ))}
            </ul>
          )}
        </>
      )}
    </div>
  );
}
