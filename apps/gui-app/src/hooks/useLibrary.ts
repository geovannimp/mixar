import { useCallback, useEffect, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { toastManager } from "@/components/ui/toast";
import { getLibraryTransport } from "@/lib/library/transport";
import { decodeEvtBody, decodeWire, toTrackSummary } from "@/lib/library/wire";
import type { AddFolderCollectionResult, CollectionSummary, TrackSummary } from "@/types";

const libraryTransport = getLibraryTransport();

export function useLibrary() {
  const [collections, setCollections] = useState<CollectionSummary[]>([]);
  const [selectedCollectionId, setSelectedCollectionId] = useState<string | null>(null);
  const [tracks, setTracks] = useState<TrackSummary[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [analyzingTrackId, setAnalyzingTrackId] = useState<string | null>(null);

  const refreshCollections = useCallback(async () => {
    const next = await libraryTransport.listCollections();
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
    const next = await libraryTransport.listCollectionTracks(collectionId);
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

  useEffect(() => {
    let cancelled = false;
    let unsubscribe: (() => void) | undefined;

    void libraryTransport
      .subscribe((bytes) => {
        let message;
        try {
          message = decodeWire(bytes);
        } catch {
          return;
        }
        let body;
        try {
          body = decodeEvtBody(message.body);
        } catch {
          return;
        }
        switch (body.type) {
          case "track_analyzed": {
            const updated = toTrackSummary(body.track);
            setTracks((current) =>
              current.map((track) => (track.id === updated.id ? updated : track)),
            );
            setAnalyzingTrackId((current) => (current === updated.id ? null : current));
            break;
          }
          case "error": {
            setError(body.message);
            if (body.track_id) {
              setAnalyzingTrackId((current) => (current === body.track_id ? null : current));
            } else {
              setAnalyzingTrackId(null);
            }
            break;
          }
          case "notice":
          case "empty":
            break;
          default: {
            const _exhaustive: never = body;
            void _exhaustive;
            break;
          }
        }
      })
      .then((unsub) => {
        if (cancelled) {
          unsub();
          return;
        }
        unsubscribe = unsub;
      });

    return () => {
      cancelled = true;
      unsubscribe?.();
    };
  }, []);

  const addFolderCollectionFromPath = useCallback(
    async (folderPath: string) => {
      setBusy(true);
      setError(null);
      try {
        const result: AddFolderCollectionResult =
          await libraryTransport.addFolderCollection(folderPath);
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
      await libraryTransport.publish("library", "analyze_track", {
        track_id: trackId,
        force: false,
      });
    } catch (err) {
      setError(String(err));
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
