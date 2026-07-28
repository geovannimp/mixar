import { describe, expect, it } from "vitest";

import { createLibraryTransport, getLibraryTransport } from "@/lib/library/transport";

describe("library transport", () => {
  it("creates a memory transport with empty read defaults", async () => {
    const transport = createLibraryTransport({ backend: "memory" });

    await expect(transport.listCollections()).resolves.toEqual([]);
    await expect(transport.listCollectionTracks("collection-1")).resolves.toEqual([]);
    await expect(transport.resolveTracksForPaths(["/music/a.mp3"])).resolves.toEqual([]);
    await expect(
      transport.getTrackArtwork({
        trackId: "track-1",
        path: null,
      }),
    ).resolves.toBeNull();
  });

  it("memoizes the shared transport", () => {
    expect(getLibraryTransport()).toBe(getLibraryTransport());
  });
});
