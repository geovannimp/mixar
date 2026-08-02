import { splitPathBreadcrumbs } from "@/lib/driveVolumes";
import type { DirectoryListing, VolumeInfo } from "@/types";
import { DrivePathBreadcrumbs } from "./DrivePathBreadcrumbs";
import { DriveFolderRow } from "./DriveFolderRow";
import { DriveVolumeList } from "./DriveSelector";

interface DriveBrowserProps {
  volumes: VolumeInfo[];
  selectedVolume: VolumeInfo | null;
  listing: DirectoryListing | null;
  busy: boolean;
  onSelectVolume: (path: string) => void;
  onOpenDirectory: (path: string) => void;
  onCreateCollectionFromFolder: (folderPath: string) => void;
}

export function DriveBrowser({
  volumes,
  selectedVolume,
  listing,
  busy,
  onSelectVolume,
  onOpenDirectory,
  onCreateCollectionFromFolder,
}: DriveBrowserProps) {
  const { ancestors, current } = listing
    ? splitPathBreadcrumbs(listing.path, selectedVolume)
    : { ancestors: [], current: null };

  return (
    <div className="flex min-h-0 flex-1 flex-col gap-2">
      {!listing ? (
        <DriveVolumeList volumes={volumes} busy={busy} onSelectVolume={onSelectVolume} />
      ) : (
        <ul className="flex min-h-0 flex-1 flex-col gap-0.5 overflow-y-auto">
          {current && (
            <li>
              <DriveFolderRow
                busy={busy}
                title={current.path}
                selected
                label={
                  <div className="flex min-w-0 flex-wrap items-center gap-x-0.5 gap-y-0">
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
                }
                onCreateCollection={() => onCreateCollectionFromFolder(current.path)}
              />
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
                <DriveFolderRow
                  busy={busy}
                  title={directory.path}
                  indented
                  label={<span className="truncate text-sm leading-5">{directory.name}</span>}
                  onOpen={() => onOpenDirectory(directory.path)}
                  onCreateCollection={() => onCreateCollectionFromFolder(directory.path)}
                />
              </li>
            ))
          )}
        </ul>
      )}
    </div>
  );
}
