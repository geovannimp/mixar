import { useCallback, useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { findActiveVolume } from "@/lib/driveVolumes";
import type { DirectoryListing, VolumeInfo } from "@/types";

export function useDriveBrowser() {
  const [volumes, setVolumes] = useState<VolumeInfo[]>([]);
  const [currentPath, setCurrentPath] = useState<string | null>(null);
  const [listing, setListing] = useState<DirectoryListing | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const refreshVolumes = useCallback(async () => {
    const next = await invoke<VolumeInfo[]>("list_fs_volumes");
    setVolumes(next);
  }, []);

  const browsePath = useCallback(async (path: string) => {
    setBusy(true);
    setError(null);
    try {
      const next = await invoke<DirectoryListing>("browse_fs_directory", { path });
      setCurrentPath(next.path);
      setListing(next);
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  }, []);

  const openVolume = useCallback(
    async (path: string) => {
      await browsePath(path);
    },
    [browsePath],
  );

  const openDirectory = useCallback(
    async (path: string) => {
      await browsePath(path);
    },
    [browsePath],
  );

  const goUp = useCallback(async () => {
    if (!listing?.parent) {
      return;
    }
    await browsePath(listing.parent);
  }, [browsePath, listing]);

  const selectedVolume = useMemo(
    () => findActiveVolume(volumes, currentPath),
    [volumes, currentPath],
  );

  useEffect(() => {
    refreshVolumes().catch((err: unknown) => {
      setError(String(err));
    });
  }, [refreshVolumes]);

  return {
    volumes,
    currentPath,
    listing,
    selectedVolume,
    error,
    busy,
    openVolume,
    openDirectory,
    goUp,
    refreshVolumes,
  };
}
