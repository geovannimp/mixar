import { describe, expect, it } from "vitest";
import { encodeEvtBody, encodeWire } from "@/lib/library/wire";
import { useLibraryStore } from "@/stores/libraryStore";

describe("libraryStore", () => {
  it("applies track_updated and hot_cues_changed for one track", () => {
    useLibraryStore.setState({ tracks: {} });

    useLibraryStore.getState().applyBusBytes(
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

    useLibraryStore.getState().applyBusBytes(
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
});
