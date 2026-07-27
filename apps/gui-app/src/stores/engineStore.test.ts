import { beforeEach, describe, expect, it } from "vitest";
import { encodeEvtBody, encodeWire } from "@/lib/engine/wire";
import { DEFAULT_DECK_A, DEFAULT_DECK_B } from "@/stores/defaultDeck";
import { DEFAULT_SAMPLER_STATUS } from "@/stores/defaultSampler";
import { useEngineStore } from "@/stores/engineStore";
import type { EngineStatus } from "@/types";

function makeStatus(): EngineStatus {
  return {
    running: true,
    backend: "cpal",
    sample_rate: 48_000,
    crossfader: 0.5,
    cue_mix: 0,
    master_cue: false,
    decks: [DEFAULT_DECK_A, DEFAULT_DECK_B],
    sampler: DEFAULT_SAMPLER_STATUS,
  };
}

function encodeDeckUpdated(deckId: number, playing: boolean, revision: number): Uint8Array {
  return encodeWire({
    origin: { deck: deckId },
    kind: "updated",
    revision,
    action_timestamp_ms: 0,
    body: encodeEvtBody({
      type: "deck_updated",
      id: deckId,
      playing,
      volume: 1,
      speed: 1,
      eq: { low: 0, mid: 0, high: 0 },
      filter_db: 0,
      gain_trim_db: 0,
      headphone_cue: false,
      sync_mode: "off",
      cue_point_secs: null,
      quantize: true,
      active_loop: null,
      position_secs: 0,
      duration_secs: 120,
    }),
  });
}

describe("useEngineStore revision guards", () => {
  beforeEach(() => {
    useEngineStore.setState({
      status: makeStatus(),
      revision: 0,
      busRevision: 0,
      busyDecks: [false, false],
      starting: false,
    });
  });

  it("applies bus updated when legacy revision is higher", () => {
    useEngineStore.getState().applyEvent({
      type: "status",
      revision: 100,
      status: makeStatus(),
    });

    expect(useEngineStore.getState().revision).toBe(100);
    expect(useEngineStore.getState().status?.decks[0]?.playing).toBe(false);

    useEngineStore.getState().applyBusBytes(encodeDeckUpdated(0, true, 1));

    const state = useEngineStore.getState();
    expect(state.busRevision).toBe(1);
    expect(state.revision).toBe(100);
    expect(state.status?.decks[0]?.playing).toBe(true);
  });
});
