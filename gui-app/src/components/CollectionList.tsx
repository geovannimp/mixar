import { Folder, FolderOpen } from "lucide-react";
import { buttonIcon } from "@/lib/ui";
import type { CollectionSummary } from "@/types";

interface CollectionListProps {
  collections: CollectionSummary[];
  selectedCollectionId: string | null;
  onSelectCollection: (collectionId: string) => void;
  onBrowseFolder?: (folderPath: string) => void;
}

export function CollectionList({
  collections,
  selectedCollectionId,
  onSelectCollection,
  onBrowseFolder,
}: CollectionListProps) {
  if (collections.length === 0) {
    return (
      <p className="rounded border border-dashed border-white/10 px-3 py-6 text-sm text-zinc-500">
        No collections yet. Add a folder to scan audio files.
      </p>
    );
  }

  return (
    <ul className="flex flex-col gap-0.5">
      {collections.map((collection) => {
        const selected = collection.id === selectedCollectionId;
        const showBrowse = collection.kind === "folder" && collection.path && onBrowseFolder;

        return (
          <li key={collection.id}>
            <div
              className={
                selected
                  ? "flex items-center gap-2 rounded border-l-2 border-l-emerald-400 bg-emerald-500/10 px-3 py-2"
                  : "flex items-center gap-2 rounded border-l-2 border-l-transparent px-3 py-2 hover:bg-white/5"
              }
            >
              <button
                type="button"
                className="min-w-0 flex-1 text-left"
                onClick={() => onSelectCollection(collection.id)}
              >
                <span className="flex items-start gap-2">
                  {collection.kind === "folder" && (
                    <Folder className="mt-0.5 size-4 shrink-0 text-zinc-500" aria-hidden />
                  )}
                  <span className="min-w-0 flex-1">
                    <span className="block truncate text-sm font-medium">{collection.name}</span>
                    <span className="mt-0.5 block truncate text-xs text-zinc-500">
                      {collection.track_count} tracks
                    </span>
                  </span>
                </span>
              </button>
              {showBrowse ? (
                <button
                  type="button"
                  className={`${buttonIcon} shrink-0 border-white/10 bg-white/5 text-zinc-400 hover:bg-white/10 hover:text-zinc-200`}
                  title="Browse folder in Drive"
                  aria-label={`Browse ${collection.name} in Drive`}
                  onClick={(event) => {
                    event.stopPropagation();
                    onBrowseFolder(collection.path!);
                  }}
                >
                  <FolderOpen className="size-3.5" aria-hidden />
                </button>
              ) : null}
            </div>
          </li>
        );
      })}
    </ul>
  );
}
