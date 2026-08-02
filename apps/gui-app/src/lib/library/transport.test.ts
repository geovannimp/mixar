import { describe, expect, it } from "vitest";

import {
  createLibraryTransport,
  getLibraryTransport,
  setLibraryTransportForTests,
  toTauriRenderWaveformLaneArgs,
} from "@/lib/library/transport";
import { decodeEvtBody, decodeWire } from "@/lib/library/wire";

describe("library transport", () => {
  it("truncates waveform lane i32 fields so Tauri serde accepts tile centers", () => {
    // Tile math yields floats (odd tileMs / 2, duration/tiles); Rust cmd takes i32.
    const args = toTauriRenderWaveformLaneArgs({
      trackId: "t1",
      path: null,
      width: 1125.7,
      height: 55.2,
      positionMs: 11_312.5,
      visibleMs: 22_222.222_222_222_223,
      bufferRatio: 0,
      includeDetail: true,
      includeBeatGrid: true,
      eqLowDb: 0,
      eqMidDb: 0,
      eqHighDb: 0,
    });

    expect(args.width).toBe(1125);
    expect(args.height).toBe(55);
    expect(args.positionMs).toBe(11_312);
    expect(args.visibleMs).toBe(22_222);
    expect(Number.isInteger(args.positionMs)).toBe(true);
    expect(Number.isInteger(args.visibleMs)).toBe(true);
  });

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

  it("memory publish analyze emits track_analyzed to subscribers", async () => {
    const transport = createLibraryTransport({ backend: "memory" });
    setLibraryTransportForTests(transport);

    const seen: string[] = [];
    const unsub = await transport.subscribe((bytes) => {
      const message = decodeWire(bytes);
      const body = decodeEvtBody(message.body);
      if (body.type === "track_analyzed") {
        seen.push(body.track.id);
      }
    });

    await transport.publish("library", "analyze_track", {
      track_id: "track-42",
      force: false,
    });

    expect(seen).toEqual(["track-42"]);
    unsub();
    setLibraryTransportForTests(null);
  });

  it("subscribe filter matches kind and origin", async () => {
    const transport = createLibraryTransport({ backend: "memory" });

    const matched: string[] = [];
    const skipped: string[] = [];
    const unsubMatch = await transport.subscribe(
      (bytes) => {
        matched.push(decodeEvtBody(decodeWire(bytes).body).type);
      },
      { kind: "track_analyzed", origin: { track: "track-42" } },
    );
    const unsubSkip = await transport.subscribe(
      (bytes) => {
        skipped.push(decodeEvtBody(decodeWire(bytes).body).type);
      },
      { kind: "error" },
    );

    await transport.publish("library", "analyze_track", {
      track_id: "track-42",
      force: false,
    });

    expect(matched).toEqual(["track_analyzed"]);
    expect(skipped).toEqual([]);
    unsubMatch();
    unsubSkip();
  });
});
