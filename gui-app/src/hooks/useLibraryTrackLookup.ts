import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { ResolvedLibraryTrack, TrackSummary } from "../types";

export function useLibraryTrackLookup(paths: string[]) {
  const [resolvedByPath, setResolvedByPath] = useState<Record<string, TrackSummary>>({});

  const pathKey = paths.join("\0");

  useEffect(() => {
    if (paths.length === 0) {
      setResolvedByPath({});
      return;
    }

    let cancelled = false;
    const handle = window.setTimeout(() => {
      invoke<ResolvedLibraryTrack[]>("resolve_library_tracks_for_paths", {
        paths,
      })
        .then((entries) => {
          if (cancelled) {
            return;
          }
          const next: Record<string, TrackSummary> = {};
          for (const entry of entries) {
            next[entry.request_path] = entry.track;
          }
          setResolvedByPath(next);
        })
        .catch(() => {
          if (!cancelled) {
            setResolvedByPath({});
          }
        });
    }, 200);

    return () => {
      cancelled = true;
      window.clearTimeout(handle);
    };
  }, [pathKey, paths]);

  const upsertResolvedTrack = useCallback((track: TrackSummary) => {
    setResolvedByPath((current) => {
      const next = { ...current };
      for (const [requestPath, existing] of Object.entries(current)) {
        if (existing.id === track.id) {
          next[requestPath] = track;
        }
      }
      next[track.path] = track;
      return next;
    });
  }, []);

  return { resolvedByPath, upsertResolvedTrack };
}
