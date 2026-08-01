import { useEffect } from "react";
import { useShallow } from "zustand/react/shallow";
import { selectCollectionTracks, useLibraryStore } from "@/stores/libraryStore";
import type { TrackSummary } from "@/types";

export function useCollectionTracks(collectionId: string | null | undefined): {
  tracks: TrackSummary[];
} {
  const { tracks, loadCollectionTracks } = useLibraryStore(
    useShallow((state) => ({
      tracks: selectCollectionTracks(state, collectionId),
      loadCollectionTracks: state.loadCollectionTracks,
    })),
  );

  useEffect(() => {
    if (!collectionId) {
      return;
    }
    loadCollectionTracks(collectionId).catch((err: unknown) => {
      useLibraryStore.setState({ error: String(err) });
    });
  }, [collectionId, loadCollectionTracks]);

  return { tracks };
}
