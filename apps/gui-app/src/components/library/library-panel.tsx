import { useCallback, useEffect, useMemo, useState } from "react";
import { Input } from "@/components/ui/input";
import { ResizableHandle, ResizablePanel, ResizablePanelGroup } from "@/components/ui/resizable";
import { useCollectionTracks } from "@/hooks/library/use-collection-tracks";
import { useCollections } from "@/hooks/library/use-collections";
import { useDriveBrowser } from "@/hooks/library/use-drive-browser";
import { useLibraryTrackLookup } from "@/hooks/library/use-library-track-lookup";
import { useSettings } from "@/hooks/use-settings";
import { useTrack } from "@/hooks/library/use-track";
import { libraryRowFromFile, libraryRowFromTrack } from "@/lib/library-table";
import { DEFAULT_LIBRARY_TABLE_COLUMNS } from "@/lib/library-table";
import { normalizeAppSettings } from "@/lib/bus-settings";
import { buttonIcon } from "@/lib/ui";
import { toTrackSummaryView, useLibraryStore } from "@/stores/library-store";
import type { LibrarySourceTab, LibraryTableRow } from "@/types";
import { CollectionList } from "./collection-list";
import { DriveBrowser } from "./drive-browser";
import { DriveSelector } from "./drive-selector";
import { LibraryPane } from "./library-pane";
import { LibrarySourceTabs } from "./library-source-tabs";
import { LibraryTrackTable } from "./library-track-table";
import { MessageBanner } from "./message-banner";
import { engineActions } from "@/stores/engine-store";
import { useEngineBusy } from "@/hooks/engine/use-engine-busy";
import { useEngineRunning } from "@/hooks/engine/use-engine-running";

export function LibraryPanel() {
  const engineRunning = useEngineRunning();
  const engineBusy = useEngineBusy();
  const { loadLibraryTrackToDeck, loadPathToDeck } = engineActions;
  const [sourceTab, setSourceTab] = useState<LibrarySourceTab>("collections");
  const [selectedCollectionId, setSelectedCollectionId] = useState<string | null>(null);
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
    volumes,
    currentPath,
    listing,
    selectedVolume,
    error: driveError,
    busy: driveBusy,
    openVolume,
    openDirectory,
  } = useDriveBrowser();

  const driveFilePaths = useMemo(
    () => (sourceTab === "drive" ? (listing?.audio_files ?? []).map((file) => file.path) : []),
    [listing?.audio_files, sourceTab],
  );

  const { resolvedByPath, upsertResolvedTrack } = useLibraryTrackLookup(driveFilePaths);

  const {
    collections,
    busy: libraryBusy,
    error: libraryError,
    addCollection,
    addCollectionFromPath,
  } = useCollections();
  const { tracks } = useCollectionTracks(selectedCollectionId);
  const { analyse } = useTrack(null);
  const analyzingTrackId = useLibraryStore((state) => state.analyzingTrackId);
  const focusedTrackRowIndex = useLibraryStore((state) => state.focusedTrackRowIndex);
  const setTrackFocusRowCount = useLibraryStore((state) => state.setTrackFocusRowCount);
  const setFocusedLoad = useLibraryStore((state) => state.setFocusedLoad);

  const collectionIdsKey = collections.map((collection) => collection.id).join("\0");

  useEffect(() => {
    const ids = collectionIdsKey ? collectionIdsKey.split("\0") : [];
    if (ids.length === 0) {
      setSelectedCollectionId(null);
      return;
    }
    setSelectedCollectionId((current) => {
      if (current && ids.includes(current)) {
        return current;
      }
      return ids[0] ?? null;
    });
  }, [collectionIdsKey]);

  useEffect(() => {
    return useLibraryStore.subscribe((state, prev) => {
      if (state.lastAnalyzedTrackId && state.lastAnalyzedTrackId !== prev.lastAnalyzedTrackId) {
        const track = state.tracks[state.lastAnalyzedTrackId];
        if (track) {
          upsertResolvedTrack(toTrackSummaryView(track));
        }
      }
    });
  }, [upsertResolvedTrack]);

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
      sampler_play_mode: "oneshot",
      sampler_strip_route: "before",
      deck_default_sampler_bank_id: [null, null],
      default_top_jog_mode: "vinyl",
      default_outer_jog_mode: "pitch_bend",
    },
  );

  const leftPaneTitle = sourceTab === "collections" ? "Collections" : "Browse";

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

  useEffect(() => {
    const row = tableRows[focusedTrackRowIndex];
    if (!row) {
      setFocusedLoad(null);
      return;
    }
    if (row.source === "library") {
      setFocusedLoad({ trackId: row.track.id });
      return;
    }
    if (row.libraryTrack) {
      setFocusedLoad({ trackId: row.libraryTrack.id });
      return;
    }
    setFocusedLoad({ path: row.file.path });
  }, [focusedTrackRowIndex, setFocusedLoad, tableRows]);

  const handleAnalyze = useCallback(
    (trackId: string) => {
      void analyse(trackId);
    },
    [analyse],
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
      void addCollectionFromPath(folderPath).then((result) => {
        if (result) {
          setSelectedCollectionId(result.collection.id);
        }
      });
    },
    [addCollectionFromPath],
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
                    onClick={() => {
                      void addCollection().then((result) => {
                        if (result) {
                          setSelectedCollectionId(result.collection.id);
                        }
                      });
                    }}
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
              focusedRowIndex={focusedTrackRowIndex}
              onVisibleRowCountChange={setTrackFocusRowCount}
              onLoadToDeck={handleLoadRow}
              onAnalyze={handleAnalyze}
            />
          </LibraryPane>
        </ResizablePanel>
      </ResizablePanelGroup>
    </section>
  );
}
