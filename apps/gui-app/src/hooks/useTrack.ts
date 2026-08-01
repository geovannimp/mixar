import { useEffect } from "react";
import { useShallow } from "zustand/react/shallow";
import { getLibraryTransport } from "@/lib/library/transport";
import {
  getLibraryTrack,
  useLibraryTrackStore,
  type LibraryTrack,
} from "@/stores/libraryTrackStore";

const libraryTransport = getLibraryTransport();

let busStarted: Promise<void> | null = null;

function ensureLibraryTrackBus(): Promise<void> {
  busStarted ??= (async () => {
    await libraryTransport.subscribe((bytes) => {
      useLibraryTrackStore.getState().applyBusBytes(bytes);
    });
  })();
  return busStarted;
}

/** Library-owned track fields (metadata + cues/loops). Engine keeps transport/mix. */
export function useTrack(trackId: string | null | undefined): LibraryTrack | null {
  const track = useLibraryTrackStore(
    useShallow((state) => (trackId ? (state.tracks[trackId] ?? null) : null)),
  );

  useEffect(() => {
    let cancelled = false;
    void (async () => {
      await ensureLibraryTrackBus();
      if (cancelled || !trackId) {
        return;
      }
      try {
        await libraryTransport.publish("library", "refresh_track", { track_id: trackId });
      } catch {
        // Refresh is best-effort; store may already have collection/analyze data.
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [trackId]);

  return track;
}

export { getLibraryTrack };
