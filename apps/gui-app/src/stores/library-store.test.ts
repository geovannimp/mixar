import { beforeEach, describe, expect, it, vi } from "vitest";
import { setFocusedLoadResolver } from "@/lib/library/focused-load";
import { encodeEvtBody, encodeWire } from "@/lib/library/wire";
import { applyLibraryBusBytesForTests, useLibraryStore } from "@/stores/library-store";

const mocks = vi.hoisted(() => ({
  loadLibraryTrackToDeck: vi.fn((_deckId: number, _trackId: string) => Promise.resolve()),
  loadPathToDeck: vi.fn((_deckId: number, _path: string) => Promise.resolve()),
}));

vi.mock("@/stores/engine-store", () => ({
  engineActions: {
    loadLibraryTrackToDeck: mocks.loadLibraryTrackToDeck,
    loadPathToDeck: mocks.loadPathToDeck,
  },
}));

describe("libraryStore", () => {
  beforeEach(() => {
    mocks.loadLibraryTrackToDeck.mockClear();
    mocks.loadPathToDeck.mockClear();
    setFocusedLoadResolver(null);
  });

  it("applies track_updated and hot_cues_changed for one track", () => {
    useLibraryStore.setState({ tracks: {} });

    applyLibraryBusBytesForTests(
      encodeWire({
        origin: { track: "t1" },
        kind: "track_updated",
        revision: 1,
        action_timestamp_ms: 0,
        body: encodeEvtBody({
          type: "track_updated",
          track: {
            id: "t1",
            display_name: "Artist — Title",
            artist: "Artist",
            title: "Title",
            album: null,
            genre: null,
            bpm: 120,
            key: "8A",
            duration_ms: 180_000,
            path: "/music/a.wav",
          },
        }),
      }),
    );

    applyLibraryBusBytesForTests(
      encodeWire({
        origin: { track: "t1" },
        kind: "hot_cues_changed",
        revision: 2,
        action_timestamp_ms: 0,
        body: encodeEvtBody({
          type: "hot_cues_changed",
          track_id: "t1",
          hot_cues: [
            {
              slot: 1,
              position_ms: 500,
              loop_length_beats: null,
              color: null,
              label: null,
            },
          ],
        }),
      }),
    );

    const track = useLibraryStore.getState().tracks.t1;
    expect(track?.title).toBe("Title");
    expect(track?.artist).toBe("Artist");
    expect(track?.hot_cues).toEqual([
      { slot: 1, position_ms: 500, loop_length_beats: null, color: null, label: null },
    ]);
  });

  it("navigate advances focusedTrackRowIndex by delta", () => {
    useLibraryStore.setState({
      focusedTrackRowIndex: 0,
      trackFocusRowCount: 5,
    });

    applyLibraryBusBytesForTests(
      encodeWire({
        origin: "library_navigation",
        kind: "navigate",
        revision: 1,
        action_timestamp_ms: 0,
        body: encodeEvtBody({ type: "navigate", delta: 2 }),
      }),
    );

    expect(useLibraryStore.getState().focusedTrackRowIndex).toBe(2);
  });

  it("load resolves focused library track id", () => {
    setFocusedLoadResolver(() => ({ trackId: "t1" }));

    applyLibraryBusBytesForTests(
      encodeWire({
        origin: "library_navigation",
        kind: "load",
        revision: 1,
        action_timestamp_ms: 0,
        body: encodeEvtBody({ type: "load", deck: 0 }),
      }),
    );

    expect(mocks.loadLibraryTrackToDeck).toHaveBeenCalledWith(0, "t1");
    expect(mocks.loadPathToDeck).not.toHaveBeenCalled();
  });

  it("load resolves focused filesystem path", () => {
    setFocusedLoadResolver(() => ({ path: "/music/b.wav" }));

    applyLibraryBusBytesForTests(
      encodeWire({
        origin: "library_navigation",
        kind: "load",
        revision: 1,
        action_timestamp_ms: 0,
        body: encodeEvtBody({ type: "load", deck: 1 }),
      }),
    );

    expect(mocks.loadPathToDeck).toHaveBeenCalledWith(1, "/music/b.wav");
    expect(mocks.loadLibraryTrackToDeck).not.toHaveBeenCalled();
  });
});
