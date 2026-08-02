import { createMemoryLibraryTransport } from "@/lib/library/memoryTransport";
import { createTauriLibraryTransport } from "@/lib/library/tauriTransport";
import type { CmdKind, Origin, SubscribeFilter } from "@/lib/library/wire";
import type {
  AddFolderCollectionResult,
  CollectionSummary,
  ResolvedLibraryTrack,
  TrackSummary,
  WaveformFrame,
} from "@/types";

export type { SubscribeFilter };

export interface RenderWaveformLaneRequest {
  trackId: string | null;
  path: string | null;
  width: number;
  height: number;
  positionMs: number;
  visibleMs: number;
  bufferRatio: number;
  includeDetail: boolean;
  includeBeatGrid: boolean;
  eqLowDb: number;
  eqMidDb: number;
  eqHighDb: number;
}

/** Tauri `render_waveform_lane` takes i32 px/ms fields; tile math often yields floats. */
export function toTauriRenderWaveformLaneArgs(
  request: RenderWaveformLaneRequest,
): RenderWaveformLaneRequest {
  return {
    ...request,
    width: Math.trunc(request.width),
    height: Math.trunc(request.height),
    positionMs: Math.trunc(request.positionMs),
    visibleMs: Math.trunc(request.visibleMs),
  };
}

export interface GetTrackArtworkRequest {
  trackId: string | null;
  path: string | null;
}

export interface LibraryTransport {
  listCollections(): Promise<CollectionSummary[]>;
  listCollectionTracks(collectionId: string): Promise<TrackSummary[]>;
  addFolderCollection(folderPath: string): Promise<AddFolderCollectionResult>;
  resolveTracksForPaths(paths: string[]): Promise<ResolvedLibraryTrack[]>;
  renderWaveformLane(request: RenderWaveformLaneRequest): Promise<WaveformFrame>;
  getTrackArtwork(request: GetTrackArtworkRequest): Promise<string | null>;
  /** `fields` are CmdBody payload fields only — body `type` is derived from `kind`. */
  publish(origin: Origin, kind: CmdKind, fields?: Record<string, unknown>): Promise<void>;
  /**
   * Resolves after the host listener is registered.
   * Optional `filter` matches origin and/or kind (client-side; host still forwards all evt).
   */
  subscribe(handler: (message: Uint8Array) => void, filter?: SubscribeFilter): Promise<() => void>;
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
      throw new Error(`Unknown library transport backend: ${String(_exhaustive)}`);
    }
  }
}

let sharedTransport: LibraryTransport | null = null;

export function getLibraryTransport(): LibraryTransport {
  sharedTransport ??= createLibraryTransport();
  return sharedTransport;
}

/** Test helper: swap the shared transport (pass `null` to clear). */
export function setLibraryTransportForTests(transport: LibraryTransport | null): void {
  sharedTransport = transport;
}
