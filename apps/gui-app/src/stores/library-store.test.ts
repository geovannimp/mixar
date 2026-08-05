import { beforeEach, describe, expect, it, vi } from "vitest";
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

  it("navigate_next advances focusedTrackRowIndex", () => {
    useLibraryStore.setState({
      focusedTrackRowIndex: 0,
      trackFocusRowCount: 3,
    });

    applyLibraryBusBytesForTests(
      encodeWire({
        origin: "library_navigation",
        kind: "navigate_next",
        revision: 1,
        action_timestamp_ms: 0,
        body: encodeEvtBody({ type: "empty" }),
      }),
    );

    expect(useLibraryStore.getState().focusedTrackRowIndex).toBe(1);
  });

  it("load_focused_to_deck loads library track id", () => {
    useLibraryStore.setState({ focusedLoad: { trackId: "t1" } });

    applyLibraryBusBytesForTests(
      encodeWire({
        origin: "library_navigation",
        kind: "load_focused_to_deck",
        revision: 1,
        action_timestamp_ms: 0,
        body: encodeEvtBody({ type: "load_focused_to_deck", deck: 0 }),
      }),
    );

    expect(mocks.loadLibraryTrackToDeck).toHaveBeenCalledWith(0, "t1");
    expect(mocks.loadPathToDeck).not.toHaveBeenCalled();
  });

  it("load_focused_to_deck loads filesystem path", () => {
    useLibraryStore.setState({ focusedLoad: { path: "/music/b.wav" } });

    applyLibraryBusBytesForTests(
      encodeWire({
        origin: "library_navigation",
        kind: "load_focused_to_deck",
        revision: 1,
        action_timestamp_ms: 0,
        body: encodeEvtBody({ type: "load_focused_to_deck", deck: 1 }),
      }),
    );

    expect(mocks.loadPathToDeck).toHaveBeenCalledWith(1, "/music/b.wav");
    expect(mocks.loadLibraryTrackToDeck).not.toHaveBeenCalled();
  });
});
