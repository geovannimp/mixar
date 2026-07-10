import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";

export function useTrackArtwork(
  trackId: string | null,
  path: string | null,
): string | null {
  const [artwork, setArtwork] = useState<string | null>(null);

  useEffect(() => {
    if (!trackId && !path) {
      setArtwork(null);
      return;
    }

    let cancelled = false;
    void invoke<string | null>("get_track_artwork", {
      trackId,
      path,
    })
      .then((encoded) => {
        if (cancelled) {
          return;
        }
        setArtwork(
          encoded ? `data:image/jpeg;base64,${encoded}` : null,
        );
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
