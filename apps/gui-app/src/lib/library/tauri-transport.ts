import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { toTauriRenderWaveformLaneArgs, type LibraryTransport } from "@/lib/library/transport";
import {
  actionTimestampMsFromFields,
  cmdBodyForKind,
  decodeWire,
  encodeWireCmd,
  matchesSubscribeFilter,
  type SubscribeFilter,
} from "@/lib/library/wire";
import { decodeWaveformFrame } from "@/lib/waveform-frame";
import type {
  AddFolderCollectionResult,
  CollectionSummary,
  ResolvedLibraryTrack,
  TrackSummary,
} from "@/types";

export const LIBRARY_BUS_EVENT = "library://bus";

export function createTauriLibraryTransport(): LibraryTransport {
  return {
    listCollections: () => invoke<CollectionSummary[]>("list_collections"),
    listCollectionTracks: (collectionId) =>
      invoke<TrackSummary[]>("list_collection_tracks", { collectionId }),
    addFolderCollection: (folderPath) =>
      invoke<AddFolderCollectionResult>("add_folder_collection", { folderPath }),
    resolveTracksForPaths: (paths) =>
      invoke<ResolvedLibraryTrack[]>("resolve_library_tracks_for_paths", { paths }),
    // Tauri commands take flat camelCase args, not a nested `request` object.
    // `ipc::Response` returns raw bytes (ArrayBuffer / Uint8Array / number[]).
    renderWaveformLane: async (request) => {
      const raw = await invoke<ArrayBuffer | Uint8Array | number[]>("render_waveform_lane", {
        ...toTauriRenderWaveformLaneArgs(request),
      });
      return decodeWaveformFrame(raw);
    },
    getTrackArtwork: (request) => invoke<string | null>("get_track_artwork", { ...request }),
    publish: (origin, kind, fields = {}) =>
      invoke("library_publish", {
        payload: Array.from(
          encodeWireCmd(
            origin,
            kind,
            cmdBodyForKind(kind, fields),
            0,
            actionTimestampMsFromFields(fields),
          ),
        ),
      }),
    subscribe: async (handler, filter?: SubscribeFilter) => {
      const unlisten = await listen<number[] | Uint8Array>(LIBRARY_BUS_EVENT, (event) => {
        const payload = event.payload;
        const bytes = payload instanceof Uint8Array ? payload : Uint8Array.from(payload ?? []);
        if (bytes.length === 0) {
          return;
        }
        if (filter) {
          try {
            if (!matchesSubscribeFilter(decodeWire(bytes), filter)) {
              return;
            }
          } catch {
            return;
          }
        }
        handler(bytes);
      });
      return () => {
        unlisten();
      };
    },
  };
}
