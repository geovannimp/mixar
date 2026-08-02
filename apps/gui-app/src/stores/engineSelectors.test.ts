import { beforeEach, describe, expect, it } from "vitest";
import { encode } from "@msgpack/msgpack";
import {
  selectDeckControls,
  selectDeckLevels,
  selectDeckMixerChannel,
  selectDeckOverview,
  selectDeckPosition,
  selectDeckWaveform,
  useEngineStore,
} from "@/stores/engineStore";
import { DEFAULT_DECK_A, DEFAULT_DECK_B } from "@/stores/defaultDeck";
import { DEFAULT_SAMPLER_STATUS } from "@/stores/defaultSampler";
import type { EngineStatus } from "@/types";

function packWire(origin: unknown, kind: string, revision: number, body: unknown): Uint8Array {
  return encode({
    origin,
    kind,
    revision,
    body: encode(body),
  });
}

function makeStatus(): EngineStatus {
  return {
    running: true,
    backend: "cpal",
    sample_rate: 48_000,
    crossfader: 0.5,
    cue_mix: 0,
    master_cue: false,
    decks: [
      {
        ...DEFAULT_DECK_A,
        track: "/a.wav",
        track_id: "t0",
        playing: true,
        position_ms: 1000,
        duration_ms: 180_000,
        levels: { peak_l: 0.1, peak_r: 0.1, peak_hold_l: 0.2, peak_hold_r: 0.2 },
      },
      DEFAULT_DECK_B,
    ],
    sampler: DEFAULT_SAMPLER_STATUS,
  };
}

describe("HF deck selector isolation", () => {
  beforeEach(() => {
    useEngineStore.setState({
      status: makeStatus(),
      revision: 1,
      busyDecks: [false, false],
      starting: false,
    });
  });

  it("mixer channel snapshot stays equal across levels bus updates", () => {
    const before = selectDeckMixerChannel(useEngineStore.getState(), 0);
    expect(before).not.toHaveProperty("levels");

    useEngineStore.getState().applyBusBytes(
      packWire({ deck: 0 }, "levels", 1, {
        type: "levels",
        peak_l: 0.9,
        peak_r: 0.8,
        peak_hold_l: 0.95,
        peak_hold_r: 0.85,
      }),
    );

    const after = selectDeckMixerChannel(useEngineStore.getState(), 0);
    expect(after).toEqual(before);
    expect(selectDeckLevels(useEngineStore.getState(), 0).peak_l).toBe(0.9);
  });

  it("controls / waveform / overview snapshots stay equal across position bus updates", () => {
    const state = () => useEngineStore.getState();
    const beforeControls = selectDeckControls(state(), 0);
    const beforeWaveform = selectDeckWaveform(state(), 0);
    const beforeOverview = selectDeckOverview(state(), 0);

    expect(beforeWaveform).not.toHaveProperty("position_ms");
    expect(beforeOverview).not.toHaveProperty("position_ms");

    useEngineStore.getState().applyBusBytes(
      packWire({ deck: 0 }, "position", 1, {
        type: "position",
        position_ms: 12_250,
      }),
    );

    expect(selectDeckControls(state(), 0)).toEqual(beforeControls);
    expect(selectDeckWaveform(state(), 0)).toEqual(beforeWaveform);
    expect(selectDeckOverview(state(), 0)).toEqual(beforeOverview);
    expect(selectDeckPosition(state(), 0)).toBe(12_250);
  });
});
