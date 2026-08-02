import { useEffect, useMemo } from "react";
import { useShallow } from "zustand/react/shallow";
import { toTrackSummaryView, useLibraryStore, type LibraryTrack } from "@/stores/library-store";
import type { TrackSummary } from "@/types";

export function useCollectionTracks(collectionId: string | null | undefined): {
  tracks: TrackSummary[];
} {
  const { trackIds, tracksById, loadCollectionTracks } = useLibraryStore(
    useShallow((state) => ({
      trackIds: collectionId ? (state.collections[collectionId]?.trackIds ?? EMPTY_IDS) : EMPTY_IDS,
      tracksById: state.tracks,
      loadCollectionTracks: state.loadCollectionTracks,
    })),
  );

  const tracks = useMemo(
    () =>
      trackIds
        .map((id) => tracksById[id])
        .filter((track): track is LibraryTrack => Boolean(track))
        .map(toTrackSummaryView),
    [trackIds, tracksById],
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

const EMPTY_IDS: string[] = [];
