import { invoke } from "@tauri-apps/api/core";

import type {
  AddFolderCollectionResult,
  CollectionSummary,
  ResolvedLibraryTrack,
  TrackSummary,
  WaveformFrame,
} from "@/types";
import type {
  GetTrackArtworkRequest,
  LibraryTransport,
  RenderWaveformLaneRequest,
} from "@/lib/library/transport";

export function createTauriLibraryTransport(): LibraryTransport {
  return {
    listCollections: () => invoke<CollectionSummary[]>("list_collections"),
    listCollectionTracks: (collectionId) =>
      invoke<TrackSummary[]>("list_collection_tracks", { collectionId }),
    addFolderCollection: (folderPath) =>
      invoke<AddFolderCollectionResult>("add_folder_collection", { folderPath }),
    analyzeTrack: (trackId) => invoke<TrackSummary>("analyze_library_track", { trackId }),
    resolveTracksForPaths: (paths) =>
      invoke<ResolvedLibraryTrack[]>("resolve_library_tracks_for_paths", { paths }),
    renderWaveformLane: (request: RenderWaveformLaneRequest) =>
      invoke<WaveformFrame>("render_waveform_lane", { ...request }),
    getTrackArtwork: (request: GetTrackArtworkRequest) =>
      invoke<string | null>("get_track_artwork", { ...request }),
  };
}
