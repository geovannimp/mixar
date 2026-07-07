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
      <p className="rounded border border-dashed border-white/10 px-3 py-6 text-sm text-zinc-500">
        No collections yet. Add a folder to scan audio files.
      </p>
    );
  }

  return (
    <ul className="flex flex-col gap-0.5">
      {collections.map((collection) => {
        const selected = collection.id === selectedCollectionId;
        return (
          <li key={collection.id}>
            <button
              type="button"
              className={
                selected
                  ? "w-full rounded border-l-2 border-l-emerald-400 bg-emerald-500/10 px-3 py-2 text-left"
                  : "w-full rounded border-l-2 border-l-transparent px-3 py-2 text-left hover:bg-white/5"
              }
              onClick={() => onSelectCollection(collection.id)}
            >
              <span className="block truncate text-sm font-medium">
                {collection.name}
              </span>
              <span className="mt-0.5 block truncate text-xs text-zinc-500">
                {collection.track_count} tracks
              </span>
            </button>
          </li>
        );
      })}
    </ul>
  );
}
