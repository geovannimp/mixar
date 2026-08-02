import { useEffect } from "react";
import { PhysicalPosition } from "@tauri-apps/api/dpi";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { toastManager } from "@/components/ui/toast";
import {
  clearOsFileDropHover,
  filterAudioFilePaths,
  findOsFileDropTargetAt,
  setOsFileDropHover,
} from "@/lib/os-file-drop-targets";
import { isTauriApp } from "@/lib/tauri-app";
import { applyTrackDrop, trackPayloadFromOsPath } from "@/lib/track-drag";

type DragPoint = { x: number; y: number };
type DragEnterPayload = { paths: string[]; position: DragPoint };
type DragOverPayload = { position: DragPoint };
type DragDropPayload = { paths: string[]; position: DragPoint };

async function clientPointFromPhysical(position: DragPoint): Promise<DragPoint> {
  const scale = await getCurrentWindow().scaleFactor();
  const logical = new PhysicalPosition(position.x, position.y).toLogical(scale);
  return { x: logical.x, y: logical.y };
}

/**
 * Tauri OS file drops → same zones / applyTrackDrop as library dnd-kit drops.
 * Programmatic dnd-kit `actions.start` for OS files does not reliably drive
 * overlay/collisions; hit-test the registered TrackDropZones instead.
 */
export function OsFileDropBridge() {
  useEffect(() => {
    if (!isTauriApp()) {
      return;
    }

    const unlisteners: Array<() => void> = [];
    let cancelled = false;

    void (async () => {
      const add = async <T,>(event: string, handler: (event: { payload: T }) => void) => {
        const unlisten = await listen<T>(event, handler);
        if (cancelled) {
          unlisten();
          return;
        }
        unlisteners.push(unlisten);
      };

      await add<DragEnterPayload>("tauri://drag-enter", async (event) => {
        if (cancelled) {
          return;
        }
        const point = await clientPointFromPhysical(event.payload.position);
        setOsFileDropHover(findOsFileDropTargetAt(point.x, point.y));
      });

      await add<DragOverPayload>("tauri://drag-over", async (event) => {
        if (cancelled) {
          return;
        }
        const point = await clientPointFromPhysical(event.payload.position);
        setOsFileDropHover(findOsFileDropTargetAt(point.x, point.y));
      });

      await add<unknown>("tauri://drag-leave", () => {
        if (cancelled) {
          return;
        }
        clearOsFileDropHover();
      });

      await add<DragDropPayload>("tauri://drag-drop", async (event) => {
        if (cancelled) {
          return;
        }
        clearOsFileDropHover();
        const point = await clientPointFromPhysical(event.payload.position);
        const target = findOsFileDropTargetAt(point.x, point.y);
        if (!target) {
          return;
        }

        const path = filterAudioFilePaths(event.payload.paths)[0];
        if (!path) {
          toastManager.add({
            title: "No supported audio files in drop",
            type: "warning",
          });
          return;
        }
        applyTrackDrop(target.data, trackPayloadFromOsPath(path));
      });
    })();

    return () => {
      cancelled = true;
      clearOsFileDropHover();
      for (const unlisten of unlisteners) {
        unlisten();
      }
    };
  }, []);

  return null;
}
