import { useDefaultLayout } from "react-resizable-panels";
import {
  ResizableHandle,
  ResizablePanel,
  ResizablePanelGroup,
} from "@/components/ui/resizable";
import { useLibrary } from "../hooks/useLibrary";
import { buttonIcon } from "../lib/ui";
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
    analyzingTrackId,
    setSelectedCollectionId,
    addFolderCollection,
    analyzeTrack,
  } = useLibrary();

  const selectedCollection = collections.find(
    (collection) => collection.id === selectedCollectionId,
  );

  const panelBusy = busy || engineBusy;

  const librarySplit = useDefaultLayout({
    id: "library-split-v2",
    panelIds: ["collections", "tracks"],
  });

  return (
    <section className="flex h-full min-h-0 flex-col bg-zinc-900/40">
      {(error || scanMessage) && (
        <div className="shrink-0 space-y-2 px-4 pt-3">
          {error && <MessageBanner message={error} variant="error" />}
          {scanMessage && (
            <MessageBanner message={scanMessage} variant="success" />
          )}
        </div>
      )}

      <ResizablePanelGroup
        id="library-split-v2"
        orientation="horizontal"
        className="min-h-0 flex-1"
        defaultLayout={librarySplit.defaultLayout}
        onLayoutChanged={librarySplit.onLayoutChanged}
      >
        <ResizablePanel
          id="collections"
          defaultSize="24"
          minSize="140px"
          maxSize="50"
          className="min-h-0 overflow-hidden"
        >
          <aside className="flex h-full min-h-0 flex-col">
            <div className="flex shrink-0 items-center justify-between gap-2 px-3 py-2">
              <p className="text-[10px] font-semibold uppercase tracking-widest text-zinc-600">
                Collections
              </p>
              <button
                type="button"
                className={`${buttonIcon} border-amber-500/35 bg-amber-500/12 hover:bg-amber-500/20`}
                disabled={panelBusy}
                title="Add folder collection"
                aria-label="Add folder collection"
                onClick={addFolderCollection}
              >
                +
              </button>
            </div>
            <div className="min-h-0 flex-1 overflow-y-auto px-2 pb-3">
              <CollectionList
                collections={collections}
                selectedCollectionId={selectedCollectionId}
                onSelectCollection={setSelectedCollectionId}
              />
            </div>
          </aside>
        </ResizablePanel>

        <ResizableHandle withHandle className="bg-white/6 hover:bg-emerald-500/25" />

        <ResizablePanel
          id="tracks"
          minSize="35"
          className="min-h-0 overflow-hidden"
        >
          <div className="flex h-full min-h-0 flex-col">
            <p className="shrink-0 px-3 py-2 text-[10px] font-semibold uppercase tracking-widest text-zinc-600">
              {selectedCollection ? selectedCollection.name : "Tracks"}
            </p>
            <div className="min-h-0 flex-1 overflow-auto px-2 pb-3">
              <TrackList
                tracks={tracks}
                selectedCollection={selectedCollection}
                engineRunning={engineRunning}
                busy={panelBusy}
                analyzingTrackId={analyzingTrackId}
                onLoadToDeck={onLoadToDeck}
                onAnalyze={analyzeTrack}
              />
            </div>
          </div>
        </ResizablePanel>
      </ResizablePanelGroup>
    </section>
  );
}
