import { describe, expect, it } from "vitest";
import { applyLibraryPerformanceBytes } from "@/lib/library/applyLibraryPerformance";
import { encodeEvtBody, encodeWire } from "@/lib/library/wire";
import type { EngineStatus } from "@/types";
import { getDefaultDeck } from "@/stores/defaultDeck";
import { DEFAULT_SAMPLER_STATUS } from "@/stores/defaultSampler";

function statusWithTrack(trackId: string): EngineStatus {
  const deck0 = { ...getDefaultDeck(0), track_id: trackId };
  return {
    running: true,
    backend: "null",
    sample_rate: 48000,
    crossfader: 0.5,
    cue_mix: 0,
    master_cue: false,
    master_deck: 0,
    decks: [deck0, getDefaultDeck(1)],
    sampler: DEFAULT_SAMPLER_STATUS,
  };
}

describe("applyLibraryPerformanceBytes", () => {
  it("patches only decks for the evt track id", () => {
    const current = statusWithTrack("track-a");
    current.decks[1] = { ...current.decks[1], track_id: "track-b" };

    const body = encodeEvtBody({
      type: "hot_cues_changed",
      track_id: "track-a",
      hot_cues: [
        {
          slot: 2,
          position_ms: 500,
          loop_length_beats: null,
          color: null,
          label: null,
        },
      ],
    });
    const bytes = encodeWire({
      origin: { track: "track-a" },
      kind: "hot_cues_changed",
      revision: 1,
      action_timestamp_ms: 0,
      body,
    });

    const next = applyLibraryPerformanceBytes(current, bytes);
    expect(next?.decks[0]?.hot_cues).toEqual([
      { slot: 2, position_ms: 500, loop_length_beats: null, color: null, label: null },
    ]);
    expect(next?.decks[1]?.hot_cues).toEqual([]);
  });
});
