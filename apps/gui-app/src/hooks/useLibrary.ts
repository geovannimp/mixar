import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { toastManager } from "@/components/ui/toast";
import type { AddFolderCollectionResult, CollectionSummary, TrackSummary } from "@/types";

export function useLibrary() {
  const [collections, setCollections] = useState<CollectionSummary[]>([]);
  const [selectedCollectionId, setSelectedCollectionId] = useState<string | null>(null);
  const [tracks, setTracks] = useState<TrackSummary[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [analyzingTrackId, setAnalyzingTrackId] = useState<string | null>(null);

  const refreshCollections = useCallback(async () => {
    const next = await invoke<CollectionSummary[]>("list_collections");
    setCollections(next);
    if (next.length === 0) {
      setSelectedCollectionId(null);
      setTracks([]);
      return;
    }
    setSelectedCollectionId((current) => {
      if (current && next.some((collection) => collection.id === current)) {
        return current;
      }
      return next[0]?.id ?? null;
    });
  }, []);

  const refreshTracks = useCallback(async (collectionId: string) => {
    const next = await invoke<TrackSummary[]>("list_collection_tracks", { collectionId });
    setTracks(next);
  }, []);

  useEffect(() => {
    refreshCollections().catch((err: unknown) => {
      setError(String(err));
    });
  }, [refreshCollections]);

  useEffect(() => {
    if (!selectedCollectionId) {
      setTracks([]);
      return;
    }
    refreshTracks(selectedCollectionId).catch((err: unknown) => {
      setError(String(err));
    });
  }, [selectedCollectionId, refreshTracks]);

  const addFolderCollectionFromPath = useCallback(
    async (folderPath: string) => {
      setBusy(true);
      setError(null);
      try {
        const result = await invoke<AddFolderCollectionResult>("add_folder_collection", {
          folderPath,
        });
        setSelectedCollectionId(result.collection.id);
        await refreshCollections();
        await refreshTracks(result.collection.id);
        toastManager.add({
          title: "Collection created",
          type: "success",
        });
        return result;
      } catch (err) {
        setError(String(err));
        return null;
      } finally {
        setBusy(false);
      }
    },
    [refreshCollections, refreshTracks],
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

  const analyzeTrack = useCallback(async (trackId: string) => {
    setAnalyzingTrackId(trackId);
    setError(null);
    try {
      const updated = await invoke<TrackSummary>("analyze_library_track", { trackId });
      setTracks((current) => current.map((track) => (track.id === trackId ? updated : track)));
      return updated;
    } catch (err) {
      setError(String(err));
      return null;
    } finally {
      setAnalyzingTrackId(null);
    }
  }, []);

  return {
    collections,
    selectedCollectionId,
    tracks,
    error,
    busy,
    analyzingTrackId,
    setSelectedCollectionId,
    addFolderCollection,
    addFolderCollectionFromPath,
    analyzeTrack,
  };
}
