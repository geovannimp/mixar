import type { LibraryTransport } from "@/lib/library/transport";
import {
  cmdBodyForKind,
  decodeWire,
  encodeEvtBody,
  encodeWire,
  matchesSubscribeFilter,
  type CmdKind,
  type Origin,
  type SubscribeFilter,
} from "@/lib/library/wire";

type HandlerEntry = {
  handler: (message: Uint8Array) => void;
  filter?: SubscribeFilter;
};

/** In-memory transport for tests; publish can optionally synthesize evt via subscribe. */
export function createMemoryLibraryTransport(): LibraryTransport {
  const handlers = new Set<HandlerEntry>();

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
      let message;
      try {
        message = decodeWire(evtBytes);
      } catch {
        return;
      }
      for (const entry of handlers) {
        if (!matchesSubscribeFilter(message, entry.filter)) {
          continue;
        }
        entry.handler(evtBytes);
      }
    },
    async subscribe(handler, filter?: SubscribeFilter) {
      const entry: HandlerEntry = { handler, filter };
      handlers.add(entry);
      return () => {
        handlers.delete(entry);
      };
    },
  };
}
