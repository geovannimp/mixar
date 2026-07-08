import { useState, type ReactNode } from "react";
import { useDefaultLayout } from "react-resizable-panels";
import {
  ResizableHandle,
  ResizablePanel,
  ResizablePanelGroup,
} from "@/components/ui/resizable";
import { useDriveBrowser } from "../hooks/useDriveBrowser";
import { useLibrary } from "../hooks/useLibrary";
import { buttonIcon } from "../lib/ui";
import type { LibrarySourceTab } from "../types";
import { CollectionList } from "./CollectionList";
import { DriveBrowser } from "./DriveBrowser";
import { DriveFileList } from "./DriveFileList";
import { DriveSelector } from "./DriveSelector";
import { LibraryPane } from "./LibraryPane";
import { LibrarySourceTabs } from "./LibrarySourceTabs";
import { MessageBanner } from "./MessageBanner";
import { TrackList } from "./TrackList";

interface LibraryPanelProps {
  engineRunning: boolean;
  engineBusy: boolean;
  onLoadToDeck: (deckId: number, trackId: string) => void;
  onLoadPathToDeck: (deckId: number, path: string) => void;
}

export function LibraryPanel({
  engineRunning,
  engineBusy,
  onLoadToDeck,
  onLoadPathToDeck,
}: LibraryPanelProps) {
  const [sourceTab, setSourceTab] = useState<LibrarySourceTab>("collections");

  const {
    collections,
    selectedCollectionId,
    tracks,
    scanMessage,
    error: libraryError,
    busy: libraryBusy,
    analyzingTrackId,
    setSelectedCollectionId,
    addFolderCollection,
    analyzeTrack,
  } = useLibrary();

  const {
    volumes,
    currentPath,
    listing,
    selectedVolume,
    error: driveError,
    busy: driveBusy,
    openVolume,
    openDirectory,
    goUp,
  } = useDriveBrowser();

  const selectedCollection = collections.find(
    (collection) => collection.id === selectedCollectionId,
  );

  const panelBusy = libraryBusy || engineBusy || driveBusy;
  const error = libraryError || driveError;

  const librarySplit = useDefaultLayout({
    id: "library-split-v3",
    panelIds: ["collections", "tracks"],
  });

  const rightPaneTitle =
    sourceTab === "collections"
      ? selectedCollection
        ? selectedCollection.name
        : "Tracks"
      : listing?.path ?? "Drive";

  const leftPaneTitle =
    sourceTab === "collections" ? "Collections" : "Browse";

  const addCollectionAction: ReactNode =
    sourceTab === "collections" ? (
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
    ) : null;

  return (
    <section className="flex h-full min-h-0 flex-col bg-zinc-900/40">
      {(error || scanMessage) && (
        <div className="shrink-0 space-y-2 px-4 pt-3">
          {error && <MessageBanner message={error} variant="error" />}
          {scanMessage && sourceTab === "collections" && (
            <MessageBanner message={scanMessage} variant="success" />
          )}
        </div>
      )}

      <ResizablePanelGroup
        id="library-split-v3"
        orientation="horizontal"
        className="min-h-0 flex-1"
        defaultLayout={librarySplit.defaultLayout}
        onLayoutChanged={librarySplit.onLayoutChanged}
      >
        <ResizablePanel
          id="collections"
          defaultSize="32"
          minSize="200px"
          maxSize="50"
          className="min-h-0 overflow-hidden"
        >
          <aside className="flex h-full min-h-0 flex-col">
            <LibraryPane
              title={leftPaneTitle}
              tabs={
                <LibrarySourceTabs
                  activeTab={sourceTab}
                  onTabChange={setSourceTab}
                />
              }
              headerInline={
                sourceTab === "drive" && listing ? (
                  <DriveSelector
                    volumes={volumes}
                    selectedVolume={selectedVolume}
                    disabled={driveBusy}
                    onSelectVolume={openVolume}
                  />
                ) : undefined
              }
              headerAction={addCollectionAction}
            >
              {sourceTab === "collections" ? (
                <CollectionList
                  collections={collections}
                  selectedCollectionId={selectedCollectionId}
                  onSelectCollection={setSelectedCollectionId}
                />
              ) : (
                <DriveBrowser
                  volumes={volumes}
                  selectedVolume={selectedVolume}
                  listing={listing}
                  busy={driveBusy}
                  onSelectVolume={openVolume}
                  onOpenDirectory={openDirectory}
                  onGoUp={goUp}
                />
              )}
            </LibraryPane>
          </aside>
        </ResizablePanel>

        <ResizableHandle withHandle className="bg-white/6 hover:bg-emerald-500/25" />

        <ResizablePanel
          id="tracks"
          minSize="35"
          className="min-h-0 overflow-hidden"
        >
          <LibraryPane title={rightPaneTitle} titleTooltip={rightPaneTitle}>
            {sourceTab === "collections" ? (
              <TrackList
                tracks={tracks}
                selectedCollection={selectedCollection}
                engineRunning={engineRunning}
                busy={panelBusy}
                analyzingTrackId={analyzingTrackId}
                onLoadToDeck={onLoadToDeck}
                onAnalyze={analyzeTrack}
              />
            ) : (
              <DriveFileList
                audioFiles={listing?.audio_files ?? []}
                currentPath={currentPath}
                engineRunning={engineRunning}
                busy={panelBusy}
                onLoadToDeck={onLoadPathToDeck}
              />
            )}
          </LibraryPane>
        </ResizablePanel>
      </ResizablePanelGroup>
    </section>
  );
}
