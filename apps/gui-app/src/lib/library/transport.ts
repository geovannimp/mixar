import { createMemoryLibraryTransport } from "@/lib/library/memoryTransport";
import { createTauriLibraryTransport } from "@/lib/library/tauriTransport";
import type {
  AddFolderCollectionResult,
  CollectionSummary,
  ResolvedLibraryTrack,
  TrackSummary,
  WaveformFrame,
} from "@/types";

export interface RenderWaveformLaneRequest {
  trackId: string | null;
  path: string | null;
  width: number;
  height: number;
  positionSecs: number;
  visibleSecs: number;
  bufferRatio: number;
  includeDetail: boolean;
  includeBeatGrid: boolean;
  eqLowDb: number;
  eqMidDb: number;
  eqHighDb: number;
}

export interface GetTrackArtworkRequest {
  trackId: string | null;
  path: string | null;
}

export interface LibraryTransport {
  listCollections(): Promise<CollectionSummary[]>;
  listCollectionTracks(collectionId: string): Promise<TrackSummary[]>;
  addFolderCollection(folderPath: string): Promise<AddFolderCollectionResult>;
  analyzeTrack(trackId: string): Promise<TrackSummary>;
  resolveTracksForPaths(paths: string[]): Promise<ResolvedLibraryTrack[]>;
  renderWaveformLane(request: RenderWaveformLaneRequest): Promise<WaveformFrame>;
  getTrackArtwork(request: GetTrackArtworkRequest): Promise<string | null>;
}

export type LibraryBackend = "tauri" | "memory";

export function createLibraryTransport(options?: { backend?: LibraryBackend }): LibraryTransport {
  const backend: LibraryBackend = options?.backend ?? "tauri";
  switch (backend) {
    case "memory":
      return createMemoryLibraryTransport();
    case "tauri":
      return createTauriLibraryTransport();
    default: {
      const _exhaustive: never = backend;
      throw new Error(`Unknown library transport backend: ${_exhaustive}`);
    }
  }
}

let sharedTransport: LibraryTransport | null = null;

export function getLibraryTransport(): LibraryTransport {
  sharedTransport ??= createLibraryTransport();
  return sharedTransport;
}
