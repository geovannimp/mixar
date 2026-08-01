import type { LibraryTransport } from "@/lib/library/transport";
import {
  cmdBodyForKind,
  encodeEvtBody,
  encodeWire,
  type CmdKind,
  type Origin,
} from "@/lib/library/wire";

/** In-memory transport for tests; publish can optionally synthesize evt via subscribe. */
export function createMemoryLibraryTransport(): LibraryTransport {
  const handlers = new Set<(message: Uint8Array) => void>();

  return {
    async listCollections() {
      return [];
    },
    async listCollectionTracks() {
      return [];
    },
    async addFolderCollection() {
      throw new Error("MemoryLibraryTransport.addFolderCollection is not implemented");
    },
    async resolveTracksForPaths() {
      return [];
    },
    async renderWaveformLane() {
      throw new Error("MemoryLibraryTransport.renderWaveformLane is not implemented");
    },
    async getTrackArtwork() {
      return null;
    },
    publish: async (_origin: Origin, kind: CmdKind, fields = {}) => {
      if (kind !== "analyze_track") {
        return;
      }
      const body = cmdBodyForKind(kind, fields);
      if (body.type !== "analyze_track") {
        return;
      }
      const trackId = body.track_id;
      const evtBytes = encodeWire({
        origin: { track: trackId },
        kind: "track_analyzed",
        revision: 1,
        action_timestamp_ms: 0,
        body: encodeEvtBody({
          type: "track_analyzed",
          track: {
            id: trackId,
            display_name: trackId,
            artist: null,
            title: trackId,
            album: null,
            genre: null,
            bpm: 120,
            key: null,
            duration_ms: 180_000,
            path: `/memory/${trackId}.mp3`,
          },
        }),
      });
      for (const handler of handlers) {
        handler(evtBytes);
      }
    },
    async subscribe(handler) {
      handlers.add(handler);
      return () => {
        handlers.delete(handler);
      };
    },
  };
}
