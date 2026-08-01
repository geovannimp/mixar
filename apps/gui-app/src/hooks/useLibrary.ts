import { useCallback, useEffect, useRef } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { useShallow } from "zustand/react/shallow";
import { toastManager } from "@/components/ui/toast";
import {
  selectLibraryCollections,
  selectSelectedCollectionTracks,
  useLibraryStore,
} from "@/stores/libraryStore";
import type { TrackSummary } from "@/types";

export type UseLibraryOptions = {
  onTrackAnalyzed?: (track: TrackSummary) => void;
};

export function useLibrary(options?: UseLibraryOptions) {
  const onTrackAnalyzedRef = useRef(options?.onTrackAnalyzed);
  onTrackAnalyzedRef.current = options?.onTrackAnalyzed;

  const {
    collections,
    selectedCollectionId,
    tracks,
    error,
    busy,
    analyzingTrackId,
    selectCollection,
    refreshCollections,
    loadSelectedCollectionTracks,
    addFolderCollectionFromPath: addFromPath,
    analyzeTrack,
  } = useLibraryStore(
    useShallow((state) => ({
      collections: selectLibraryCollections(state),
      selectedCollectionId: state.selectedCollectionId,
      tracks: selectSelectedCollectionTracks(state),
      error: state.error,
      busy: state.busy,
      analyzingTrackId: state.analyzingTrackId,
      selectCollection: state.selectCollection,
      refreshCollections: state.refreshCollections,
      loadSelectedCollectionTracks: state.loadSelectedCollectionTracks,
      addFolderCollectionFromPath: state.addFolderCollectionFromPath,
      analyzeTrack: state.analyzeTrack,
    })),
  );

  useEffect(() => {
    refreshCollections().catch((err: unknown) => {
      useLibraryStore.setState({ error: String(err) });
    });
  }, [refreshCollections]);

  useEffect(() => {
    if (!selectedCollectionId) {
      return;
    }
    loadSelectedCollectionTracks().catch((err: unknown) => {
      useLibraryStore.setState({ error: String(err) });
    });
  }, [selectedCollectionId, loadSelectedCollectionTracks]);

  useEffect(() => {
    return useLibraryStore.subscribe((state, prev) => {
      if (state.lastAnalyzedTrackId && state.lastAnalyzedTrackId !== prev.lastAnalyzedTrackId) {
        const track = state.tracks[state.lastAnalyzedTrackId];
        if (track) {
          onTrackAnalyzedRef.current?.({
            id: track.id,
            display_name: track.display_name,
            artist: track.artist,
            title: track.title,
            album: track.album,
            genre: track.genre,
            bpm: track.bpm,
            key: track.key,
            duration_ms: track.duration_ms,
            path: track.path,
          });
        }
      }
    });
  }, []);

  const addFolderCollectionFromPath = useCallback(
    async (folderPath: string) => {
      const result = await addFromPath(folderPath);
      if (result) {
        toastManager.add({
          title: "Collection created",
          type: "success",
        });
      }
      return result;
    },
    [addFromPath],
  );

  const addFolderCollection = useCallback(async () => {
    const selected = await open({
      directory: true,
      multiple: false,
    });
    if (typeof selected !== "string") {
      return;
    }
    await addFolderCollectionFromPath(selected);
  }, [addFolderCollectionFromPath]);

  return {
    collections,
    selectedCollectionId,
    tracks,
    error,
    busy,
    analyzingTrackId,
    setSelectedCollectionId: selectCollection,
    addFolderCollection,
    addFolderCollectionFromPath,
    analyzeTrack,
  };
}
