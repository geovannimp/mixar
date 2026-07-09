import { useCallback, useEffect, useMemo, useState } from "react";
import { Input } from "@/components/ui/input";
import {
  ResizableHandle,
  ResizablePanel,
  ResizablePanelGroup,
} from "@/components/ui/resizable";
import { useDriveBrowser } from "../hooks/useDriveBrowser";
import { useLibrary } from "../hooks/useLibrary";
import { useLibraryTrackLookup } from "../hooks/useLibraryTrackLookup";
import { useSettings } from "../hooks/useSettings";
import {
  libraryRowFromFile,
  libraryRowFromTrack,
} from "../lib/libraryTable";
import { DEFAULT_LIBRARY_TABLE_COLUMNS } from "../lib/libraryTable";
import { normalizeAppSettings } from "../lib/busSettings";
import { buttonIcon } from "../lib/ui";
import type { LibrarySourceTab, LibraryTableColumn, LibraryTableRow } from "../types";
import { CollectionList } from "./CollectionList";
import { DriveBrowser } from "./DriveBrowser";
import { DriveSelector } from "./DriveSelector";
import { LibraryPane } from "./LibraryPane";
import { LibrarySourceTabs } from "./LibrarySourceTabs";
import { LibraryTrackTable } from "./LibraryTrackTable";
import { MessageBanner } from "./MessageBanner";

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
  const [filter, setFilter] = useState("");
  const [sortColumn, setSortColumn] = useState<LibraryTableColumn>("title");
  const [sortDirection, setSortDirection] = useState<"asc" | "desc">("asc");

  const { settings, refresh: refreshSettings } = useSettings();

  const {
    collections,
    selectedCollectionId,
    tracks,
    error: libraryError,
    busy: libraryBusy,
    analyzingTrackId,
    setSelectedCollectionId,
    addFolderCollection,
    addFolderCollectionFromPath,
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
  } = useDriveBrowser();

  useEffect(() => {
    const handleFocus = () => {
      refreshSettings().catch(() => undefined);
    };
    window.addEventListener("focus", handleFocus);
    return () => window.removeEventListener("focus", handleFocus);
  }, [refreshSettings]);

  const selectedCollection = collections.find(
    (collection) => collection.id === selectedCollectionId,
  );

  const panelBusy = libraryBusy || engineBusy || driveBusy;
  const error = libraryError || driveError;
  const tableSettings = normalizeAppSettings(
    settings ?? {
      backend: "cpal",
      sample_rate: 48_000,
      buffer_size: 512,
      low_latency: false,
      resampler_quality: "medium",
      master_bus: { device_id: "default", left_channel: 1, right_channel: 2 },
      preview_enabled: false,
      preview_bus: { device_id: "default", left_channel: 3, right_channel: 4 },
      analysis_duration: "fast",
      scan_folder_tree: true,
      library_table_columns: DEFAULT_LIBRARY_TABLE_COLUMNS,
    },
  );

  const leftPaneTitle =
    sourceTab === "collections" ? "Collections" : "Browse";

  const driveFilePaths = useMemo(
    () => (sourceTab === "drive" ? (listing?.audio_files ?? []).map((file) => file.path) : []),
    [listing?.audio_files, sourceTab],
  );

  const { resolvedByPath, upsertResolvedTrack } = useLibraryTrackLookup(driveFilePaths);

  const tableRows = useMemo((): LibraryTableRow[] => {
    if (sourceTab === "collections") {
      return tracks.map(libraryRowFromTrack);
    }
    return (listing?.audio_files ?? []).map((file) =>
      libraryRowFromFile(file, resolvedByPath[file.path]),
    );
  }, [listing?.audio_files, resolvedByPath, sourceTab, tracks]);

  const emptyMessage =
    sourceTab === "collections"
      ? selectedCollection
        ? "No file tracks in this collection."
        : "Select a collection to browse tracks."
      : !currentPath
        ? "Select a drive or folder to browse audio files."
        : "No audio files in this folder.";

  const handleSortChange = useCallback((column: LibraryTableColumn) => {
    setSortColumn((current) => {
      if (current === column) {
        setSortDirection((direction) => (direction === "asc" ? "desc" : "asc"));
        return current;
      }
      setSortDirection("asc");
      return column;
    });
  }, []);

  const handleLoadRow = useCallback(
    (deckId: number, row: LibraryTableRow) => {
      if (row.source === "library") {
        void onLoadToDeck(deckId, row.track.id);
        return;
      }
      if (row.libraryTrack) {
        void onLoadToDeck(deckId, row.libraryTrack.id);
        return;
      }
      void onLoadPathToDeck(deckId, row.file.path);
    },
    [onLoadPathToDeck, onLoadToDeck],
  );

  const handleAnalyze = useCallback(
    async (trackId: string) => {
      const updated = await analyzeTrack(trackId);
      if (updated) {
        upsertResolvedTrack(updated);
      }
    },
    [analyzeTrack, upsertResolvedTrack],
  );

  const handleCreateCollectionFromFolder = useCallback(
    (folderPath: string) => {
      void addFolderCollectionFromPath(folderPath);
    },
    [addFolderCollectionFromPath],
  );

  return (
    <section className="flex h-full min-h-0 flex-col bg-zinc-900/40">
      {error && (
        <div className="shrink-0 space-y-2 px-4 pt-3">
          <MessageBanner message={error} variant="error" />
        </div>
      )}

      <ResizablePanelGroup
        id="library-split"
        orientation="horizontal"
        className="min-h-0 flex-1"
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
              headerAction={
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
                ) : null
              }
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
                  busy={panelBusy}
                  onSelectVolume={openVolume}
                  onOpenDirectory={openDirectory}
                  onCreateCollectionFromFolder={handleCreateCollectionFromFolder}
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
          <LibraryPane
            title="Tracks"
            headerInline={
              <Input
                type="search"
                size="sm"
                className="border-white/10 bg-zinc-900/80 shadow-none"
                placeholder="Filter tracks…"
                value={filter}
                onChange={(event) => setFilter(event.target.value)}
                aria-label="Filter tracks"
              />
            }
          >
            <LibraryTrackTable
              rows={tableRows}
              columns={tableSettings.library_table_columns}
              filter={filter}
              sortColumn={sortColumn}
              sortDirection={sortDirection}
              emptyMessage={emptyMessage}
              engineRunning={engineRunning}
              busy={panelBusy}
              analyzingTrackId={analyzingTrackId}
              onSortChange={handleSortChange}
              onLoadToDeck={handleLoadRow}
              onAnalyze={handleAnalyze}
            />
          </LibraryPane>
        </ResizablePanel>
      </ResizablePanelGroup>
    </section>
  );
}
