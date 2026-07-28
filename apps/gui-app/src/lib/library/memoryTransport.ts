import type { LibraryTransport } from "@/lib/library/transport";

export function createMemoryLibraryTransport(): LibraryTransport {
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
    async analyzeTrack() {
      throw new Error("MemoryLibraryTransport.analyzeTrack is not implemented");
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
  };
}
