import type { CollectionSummary } from "../types";

interface CollectionListProps {
  collections: CollectionSummary[];
  selectedCollectionId: string | null;
  onSelectCollection: (collectionId: string) => void;
}

export function CollectionList({
  collections,
  selectedCollectionId,
  onSelectCollection,
}: CollectionListProps) {
  if (collections.length === 0) {
    return (
      <p className="rounded-lg border border-dashed border-white/12 px-3 py-4 text-sm text-zinc-500">
        No collections yet. Add a folder to scan audio files.
      </p>
    );
  }

  return (
    <ul className="flex max-h-72 flex-col gap-2 overflow-y-auto">
      {collections.map((collection) => {
        const selected = collection.id === selectedCollectionId;
        return (
          <li key={collection.id}>
            <button
              type="button"
              className={
                selected
                  ? "w-full rounded-lg border border-sky-500/40 bg-sky-500/10 px-3 py-2 text-left"
                  : "w-full rounded-lg border border-white/10 bg-black/20 px-3 py-2 text-left hover:border-white/20 hover:bg-black/30"
              }
              onClick={() => onSelectCollection(collection.id)}
            >
              <span className="block truncate font-medium">{collection.name}</span>
              <span className="mt-1 block truncate text-xs text-zinc-400">
                {collection.track_count} tracks
                {collection.path ? ` · ${collection.path}` : ""}
              </span>
            </button>
          </li>
        );
      })}
    </ul>
  );
}
