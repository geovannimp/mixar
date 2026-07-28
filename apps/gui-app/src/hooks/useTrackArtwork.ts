import { useEffect, useState } from "react";
import { getLibraryTransport } from "@/lib/library/transport";

const libraryTransport = getLibraryTransport();

export function useTrackArtwork(trackId: string | null, path: string | null): string | null {
  const [artwork, setArtwork] = useState<string | null>(null);

  useEffect(() => {
    if (!trackId && !path) {
      setArtwork(null);
      return;
    }

    let cancelled = false;
    void libraryTransport
      .getTrackArtwork({ trackId, path })
      .then((encoded) => {
        if (cancelled) {
          return;
        }
        setArtwork(encoded ? `data:image/jpeg;base64,${encoded}` : null);
      })
      .catch(() => {
        if (!cancelled) {
          setArtwork(null);
        }
      });

    return () => {
      cancelled = true;
    };
  }, [trackId, path]);

  return artwork;
}
