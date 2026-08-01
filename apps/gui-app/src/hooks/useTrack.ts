import { useEffect } from "react";
import { useShallow } from "zustand/react/shallow";
import { getLibraryTransport } from "@/lib/library/transport";
import { useLibraryStore, type LibraryTrack } from "@/stores/libraryStore";

const libraryTransport = getLibraryTransport();

/** Library-owned track fields (metadata + cues/loops). Engine keeps transport/mix. */
export function useTrack(trackId: string | null | undefined): LibraryTrack | null {
  const track = useLibraryStore(
    useShallow((state) => (trackId ? (state.tracks[trackId] ?? null) : null)),
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

  return track;
}

export type { LibraryTrack };
