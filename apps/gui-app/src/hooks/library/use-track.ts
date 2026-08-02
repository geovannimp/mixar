import { useCallback, useEffect } from "react";
import { useShallow } from "zustand/react/shallow";
import { getLibraryTransport } from "@/lib/library/transport";
import { useLibraryStore, type LibraryTrack } from "@/stores/library-store";

const libraryTransport = getLibraryTransport();

/** Library-owned track fields (metadata + cues/loops). Engine keeps transport/mix. */
export function useTrack(trackId: string | null | undefined): {
  track: LibraryTrack | null;
  analyse: (id?: string) => Promise<void>;
  analyzing: boolean;
} {
  const { track, analyzing, analyzeTrack } = useLibraryStore(
    useShallow((state) => ({
      track: trackId ? (state.tracks[trackId] ?? null) : null,
      analyzing: Boolean(trackId && state.analyzingTrackId === trackId),
      analyzeTrack: state.analyzeTrack,
    })),
  );

  useEffect(() => {
    if (!trackId) {
      return;
    }
    let cancelled = false;
    void (async () => {
      try {
        await libraryTransport.publish("library", "refresh_track", { track_id: trackId });
      } catch {
        // Refresh is best-effort; store may already have collection/analyze data.
      }
      if (cancelled) {
        return;
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [trackId]);

  const analyse = useCallback(
    async (id?: string) => {
      const target = id ?? trackId;
      if (!target) {
        return;
      }
      await analyzeTrack(target);
    },
    [analyzeTrack, trackId],
  );

  return { track, analyse, analyzing };
}

export type { LibraryTrack };
