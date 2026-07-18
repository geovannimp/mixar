import { useCallback, useEffect, useMemo, useState } from "react";
import { Input } from "@/components/ui/input";
import { ResizableHandle, ResizablePanel, ResizablePanelGroup } from "@/components/ui/resizable";
import { useDriveBrowser } from "@/hooks/useDriveBrowser";
import { useLibrary } from "@/hooks/useLibrary";
import { useLibraryTrackLookup } from "@/hooks/useLibraryTrackLookup";
import { useSettings } from "@/hooks/useSettings";
import { libraryRowFromFile, libraryRowFromTrack } from "@/lib/libraryTable";
import { DEFAULT_LIBRARY_TABLE_COLUMNS } from "@/lib/libraryTable";
import { normalizeAppSettings } from "@/lib/busSettings";
import { buttonIcon } from "@/lib/ui";
import type { LibrarySourceTab, LibraryTableRow } from "@/types";
import { CollectionList } from "./CollectionList";
import { DriveBrowser } from "./DriveBrowser";
import { DriveSelector } from "./DriveSelector";
import { LibraryPane } from "./LibraryPane";
import { LibrarySourceTabs } from "./LibrarySourceTabs";
import { LibraryTrackTable } from "./LibraryTrackTable";
import { MessageBanner } from "./MessageBanner";
import { engineActions, useEngineBusy, useEngineRunning } from "@/hooks/useEngine";

export function LibraryPanel() {
  const engineRunning = useEngineRunning();
  const engineBusy = useEngineBusy();
  const { loadLibraryTrackToDeck, loadPathToDeck } = engineActions;
  const [sourceTab, setSourceTab] = useState<LibrarySourceTab>("collections");
  const [filter, setFilter] = useState("");
  const [debouncedFilter, setDebouncedFilter] = useState("");

  useEffect(() => {
    const timeout = window.setTimeout(() => {
      setDebouncedFilter(filter);
    }, 300);

    return () => window.clearTimeout(timeout);
  }, [filter]);

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
      volume_normalizer_enabled: true,
      target_lufs: -18,
    },
  );

  const leftPaneTitle = sourceTab === "collections" ? "Collections" : "Browse";

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

  const handleLoadRow = useCallback(
    (deckId: number, row: LibraryTableRow) => {
      if (row.source === "library") {
        void loadLibraryTrackToDeck(deckId, row.track.id);
        return;
      }
      if (row.libraryTrack) {
        void loadLibraryTrackToDeck(deckId, row.libraryTrack.id);
        return;
      }
      void loadPathToDeck(deckId, row.file.path);
    },
    [loadPathToDeck, loadLibraryTrackToDeck],
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

  const handleBrowseCollectionFolder = useCallback(
    (folderPath: string) => {
      setSourceTab("drive");
      void openDirectory(folderPath);
    },
    [openDirectory],
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

      <ResizablePanelGroup id="library-split" orientation="horizontal" className="min-h-0 flex-1">
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
              tabs={<LibrarySourceTabs activeTab={sourceTab} onTabChange={setSourceTab} />}
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
                  onBrowseFolder={handleBrowseCollectionFolder}
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

        <ResizablePanel id="tracks" minSize="35" className="min-h-0 overflow-hidden">
          <LibraryPane
            title="Tracks"
            scrollable={false}
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
              globalFilter={debouncedFilter}
              emptyMessage={emptyMessage}
              engineRunning={engineRunning}
              busy={panelBusy}
              analyzingTrackId={analyzingTrackId}
              onLoadToDeck={handleLoadRow}
              onAnalyze={handleAnalyze}
            />
          </LibraryPane>
        </ResizablePanel>
      </ResizablePanelGroup>
    </section>
  );
}
