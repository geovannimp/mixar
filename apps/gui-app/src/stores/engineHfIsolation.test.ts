import { beforeEach, describe, expect, it } from "vitest";
import { encode } from "@msgpack/msgpack";
import { useEngineStore } from "@/stores/engineStore";
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

/** Fields useDeckMixerChannel / useDeckControls / etc. subscribe to — must stay stable across HF ticks. */
function mixerChannelFields(deckId: number) {
  const deck = useEngineStore.getState().status?.decks[deckId];
  return {
    volume: deck?.volume,
    eq: deck?.eq,
    filter_db: deck?.filter_db,
    gain_trim_db: deck?.gain_trim_db,
    headphone_cue: deck?.headphone_cue,
  };
}

function controlsFields(deckId: number) {
  const deck = useEngineStore.getState().status?.decks[deckId];
  return {
    id: deck?.id,
    track: deck?.track,
    track_id: deck?.track_id,
    title: deck?.title,
    artist: deck?.artist,
    bpm: deck?.bpm,
    key: deck?.key,
    playing: deck?.playing,
    speed: deck?.speed,
    quantize: deck?.quantize,
    cue_point_ms: deck?.cue_point_ms,
    hot_cues: deck?.hot_cues,
    saved_loops: deck?.saved_loops,
    active_loop: deck?.active_loop,
    sync_mode: deck?.sync_mode,
    is_master: deck?.is_master,
    pad_mode: deck?.pad_mode,
    loudness_lufs: deck?.loudness_lufs,
    auto_gain_db: deck?.auto_gain_db,
    gain_trim_db: deck?.gain_trim_db,
    active_sampler_bank_id: deck?.active_sampler_bank_id,
    top_jog_mode: deck?.top_jog_mode,
    outer_jog_mode: deck?.outer_jog_mode,
    jog_touching: deck?.jog_touching,
  };
}

function waveformFields(deckId: number) {
  const deck = useEngineStore.getState().status?.decks[deckId];
  return {
    id: deck?.id,
    track: deck?.track,
    track_id: deck?.track_id,
    playing: deck?.playing,
    speed: deck?.speed,
    eq: deck?.eq,
    hot_cues: deck?.hot_cues,
    active_loop: deck?.active_loop,
    duration_ms: deck?.duration_ms,
  };
}

function overviewFields(deckId: number) {
  const deck = useEngineStore.getState().status?.decks[deckId];
  return {
    track_id: deck?.track_id,
    track: deck?.track,
    playing: deck?.playing,
    speed: deck?.speed,
    duration_ms: deck?.duration_ms,
    hot_cues: deck?.hot_cues,
  };
}

describe("HF deck field isolation", () => {
  beforeEach(() => {
    useEngineStore.setState({
      status: makeStatus(),
      revision: 1,
      busyDecks: [false, false],
      starting: false,
    });
  });

  it("mixer channel fields stay equal across levels bus updates", () => {
    const before = mixerChannelFields(0);
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

    expect(mixerChannelFields(0)).toEqual(before);
    expect(useEngineStore.getState().status?.decks[0]?.levels.peak_l).toBe(0.9);
  });

  it("controls / waveform / overview fields stay equal across position bus updates", () => {
    const beforeControls = controlsFields(0);
    const beforeWaveform = waveformFields(0);
    const beforeOverview = overviewFields(0);

    expect(beforeWaveform).not.toHaveProperty("position_ms");
    expect(beforeOverview).not.toHaveProperty("position_ms");

    useEngineStore.getState().applyBusBytes(
      packWire({ deck: 0 }, "position", 1, {
        type: "position",
        position_ms: 12_250,
      }),
    );

    expect(controlsFields(0)).toEqual(beforeControls);
    expect(waveformFields(0)).toEqual(beforeWaveform);
    expect(overviewFields(0)).toEqual(beforeOverview);
    expect(useEngineStore.getState().status?.decks[0]?.position_ms).toBe(12_250);
  });
});
