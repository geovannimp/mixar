import { Folder } from "lucide-react";
import { splitPathBreadcrumbs } from "../lib/driveVolumes";
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
}

export function DriveBrowser({
  volumes,
  selectedVolume,
  listing,
  busy,
  onSelectVolume,
  onOpenDirectory,
}: DriveBrowserProps) {
  const { ancestors, current } = listing
    ? splitPathBreadcrumbs(listing.path, selectedVolume)
    : { ancestors: [], current: null };

  return (
    <div className="flex min-h-0 flex-1 flex-col gap-2">
      {!listing ? (
        <DriveVolumeList
          volumes={volumes}
          busy={busy}
          onSelectVolume={onSelectVolume}
        />
      ) : (
        <ul className="flex min-h-0 flex-1 flex-col gap-0.5 overflow-y-auto">
          {current && (
            <li>
              <div
                className="w-full rounded border-l-2 border-l-emerald-500/60 bg-white/5 px-3 py-2"
                aria-current="location"
                title={current.path}
              >
                <div className="flex min-w-0 items-center gap-2">
                  <Folder
                    className="size-4 shrink-0 text-zinc-400"
                    aria-hidden
                  />
                  <div className="flex min-w-0 flex-1 flex-wrap items-center gap-x-0.5 gap-y-0">
                    {ancestors.length > 0 && (
                      <DrivePathBreadcrumbs
                        crumbs={ancestors}
                        listingPath={listing.path}
                        busy={busy}
                        embedded
                        onOpenDirectory={onOpenDirectory}
                      />
                    )}
                    {ancestors.length > 0 && (
                      <span className="text-sm text-zinc-600" aria-hidden>
                        /
                      </span>
                    )}
                    <span className="truncate text-sm leading-5 text-zinc-200">
                      {current.label}
                    </span>
                  </div>
                </div>
              </div>
            </li>
          )}

          {listing.directories.length === 0 ? (
            <li>
              <p className="rounded border border-dashed border-white/10 py-4 pl-6 pr-3 text-sm text-zinc-500">
                No subfolders here.
              </p>
            </li>
          ) : (
            listing.directories.map((directory) => (
              <li key={directory.path}>
                <button
                  type="button"
                  className="w-full rounded border-l-2 border-l-transparent py-2 pl-6 pr-3 text-left hover:bg-white/5"
                  disabled={busy}
                  onClick={() => onOpenDirectory(directory.path)}
                >
                  <span className="flex items-center gap-2">
                    <Folder
                      className="size-4 shrink-0 text-zinc-500"
                      aria-hidden
                    />
                    <span className="truncate text-sm leading-5">
                      {directory.name}
                    </span>
                  </span>
                </button>
              </li>
            ))
          )}
        </ul>
      )}
    </div>
  );
}
