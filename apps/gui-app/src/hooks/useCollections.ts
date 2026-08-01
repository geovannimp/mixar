import { useCallback, useEffect, useMemo } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { useShallow } from "zustand/react/shallow";
import { toastManager } from "@/components/ui/toast";
import { useLibraryStore, type LibraryCollection } from "@/stores/libraryStore";
import type { AddFolderCollectionResult, CollectionSummary } from "@/types";

export function useCollections(): {
  collections: CollectionSummary[];
  busy: boolean;
  error: string | null;
  addCollection: () => Promise<AddFolderCollectionResult | null | undefined>;
  addCollectionFromPath: (folderPath: string) => Promise<AddFolderCollectionResult | null>;
} {
  const {
    collectionIds,
    collectionsById,
    busy,
    error,
    refreshCollections,
    addFolderCollectionFromPath,
  } = useLibraryStore(
    useShallow((state) => ({
      collectionIds: state.collectionIds,
      collectionsById: state.collections,
      busy: state.busy,
      error: state.error,
      refreshCollections: state.refreshCollections,
      addFolderCollectionFromPath: state.addFolderCollectionFromPath,
    })),
  );

  const collections = useMemo(
    () =>
      collectionIds
        .map((id) => collectionsById[id])
        .filter((collection): collection is LibraryCollection => Boolean(collection)),
    [collectionIds, collectionsById],
  );

  useEffect(() => {
    refreshCollections().catch((err: unknown) => {
      useLibraryStore.setState({ error: String(err) });
    });
  }, [refreshCollections]);

  const addCollectionFromPath = useCallback(
    async (folderPath: string) => {
      const result = await addFolderCollectionFromPath(folderPath);
      if (result) {
        toastManager.add({
          title: "Collection created",
          type: "success",
        });
      }
      return result;
    },
    [addFolderCollectionFromPath],
  );

  const addCollection = useCallback(async () => {
    const selected = await open({
      directory: true,
      multiple: false,
    });
    if (typeof selected !== "string") {
      return undefined;
    }
    return addCollectionFromPath(selected);
  }, [addCollectionFromPath]);

  return {
    collections,
    busy,
    error,
    addCollection,
    addCollectionFromPath,
  };
}
