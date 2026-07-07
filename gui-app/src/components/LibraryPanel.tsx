import { buttonBase } from "../lib/ui";
import { useLibrary } from "../hooks/useLibrary";
import { CollectionList } from "./CollectionList";
import { MessageBanner } from "./MessageBanner";
import { TrackList } from "./TrackList";

interface LibraryPanelProps {
  engineRunning: boolean;
  engineBusy: boolean;
  onLoadToDeck: (deckId: number, trackId: string) => void;
}

export function LibraryPanel({
  engineRunning,
  engineBusy,
  onLoadToDeck,
}: LibraryPanelProps) {
  const {
    collections,
    selectedCollectionId,
    tracks,
    scanMessage,
    error,
    busy,
    setSelectedCollectionId,
    addFolderCollection,
  } = useLibrary();

  const selectedCollection = collections.find(
    (collection) => collection.id === selectedCollectionId,
  );

  const panelBusy = busy || engineBusy;

  return (
    <section className="flex flex-col gap-4 rounded-2xl border border-white/8 bg-white/3 p-5">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div>
          <h2 className="text-lg font-semibold">Library</h2>
          <p className="mt-1 text-sm text-zinc-400">
            Add a folder collection, then load tracks onto a deck.
          </p>
        </div>
        <button
          type="button"
          className={`${buttonBase} border-amber-500/35 bg-amber-500/12 hover:bg-amber-500/20`}
          disabled={panelBusy}
          onClick={addFolderCollection}
        >
          Add folder collection
        </button>
      </div>

      {error && <MessageBanner message={error} variant="error" />}
      {scanMessage && <MessageBanner message={scanMessage} variant="success" />}

      <div className="grid gap-4 lg:grid-cols-[minmax(220px,280px)_1fr]">
        <div className="flex flex-col gap-2">
          <p className="text-xs font-semibold uppercase tracking-wide text-zinc-500">
            Collections
          </p>
          <CollectionList
            collections={collections}
            selectedCollectionId={selectedCollectionId}
            onSelectCollection={setSelectedCollectionId}
          />
        </div>

        <div className="flex min-h-48 flex-col gap-2">
          <p className="text-xs font-semibold uppercase tracking-wide text-zinc-500">
            {selectedCollection ? `${selectedCollection.name} tracks` : "Tracks"}
          </p>
          <TrackList
            tracks={tracks}
            selectedCollection={selectedCollection}
            engineRunning={engineRunning}
            busy={panelBusy}
            onLoadToDeck={onLoadToDeck}
          />
        </div>
      </div>
    </section>
  );
}
