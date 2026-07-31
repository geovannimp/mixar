import { beforeEach, describe, expect, it, vi } from "vitest";
import { encodeEvtBody, encodeWire } from "@/lib/engine/wire";
import type { EngineTransport } from "@/lib/engine/transport";
import { setEngineTransportForTests } from "@/lib/engine/transport";
import { DEFAULT_DECK_A, DEFAULT_DECK_B } from "@/stores/defaultDeck";
import { DEFAULT_SAMPLER_STATUS } from "@/stores/defaultSampler";
import { resetEngineBusSubscriptionForTests, useEngineStore } from "@/stores/engineStore";
import type { EngineStatus } from "@/types";

vi.mock("@/components/ui/toast", () => ({
  toastManager: {
    add: vi.fn(),
    promise: vi.fn(async (p: Promise<unknown>) => p),
  },
}));

function makeStatus(running = true): EngineStatus {
  return {
    running,
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
      track: null,
      track_id: null,
      title: null,
      artist: null,
      bpm: null,
      key: null,
      playing,
      volume: 1,
      speed: 1,
      eq: { low: 0, mid: 0, high: 0 },
      filter_db: 0,
      gain_trim_db: 0,
      headphone_cue: false,
      sync_mode: "off",
      cue_point_ms: null,
      quantize: true,
      active_loop: null,
      pad_mode: "hot_cue",
      position_ms: 0,
      duration_ms: 120000,
      hot_cues: [],
      saved_loops: [],
      loudness_lufs: null,
      auto_gain_db: 0,
      active_sampler_bank_id: null,
      top_jog_mode: "vinyl",
      outer_jog_mode: "pitch_bend",
      jog_touching: false,
    }),
  });
}

describe("useEngineStore revision guards", () => {
  beforeEach(() => {
    resetEngineBusSubscriptionForTests();
    setEngineTransportForTests(null);
    useEngineStore.setState({
      status: makeStatus(),
      revision: 0,
      busyDecks: [false, false],
      starting: false,
    });
  });

  it("applies bus updated and advances revision", () => {
    expect(useEngineStore.getState().status?.decks[0]?.playing).toBe(false);

    useEngineStore.getState().applyBusBytes(encodeDeckUpdated(0, true, 1));

    const state = useEngineStore.getState();
    expect(state.revision).toBe(1);
    expect(state.status?.decks[0]?.playing).toBe(true);
  });

  it("ignores stale bus revisions", () => {
    useEngineStore.getState().applyBusBytes(encodeDeckUpdated(0, true, 5));
    useEngineStore.getState().applyBusBytes(encodeDeckUpdated(0, false, 4));

    const state = useEngineStore.getState();
    expect(state.revision).toBe(5);
    expect(state.status?.decks[0]?.playing).toBe(true);
  });
});

describe("useEngineStore ensureEngineRunning", () => {
  beforeEach(() => {
    resetEngineBusSubscriptionForTests();
    useEngineStore.setState({
      status: null,
      revision: 0,
      busyDecks: [false, false],
      starting: false,
    });
  });

  it("publishes start_engine on engine origin after bus subscribe", async () => {
    const published: Array<{ origin: unknown; kind: string }> = [];
    const transport: EngineTransport = {
      publish: async (origin, kind) => {
        published.push({ origin, kind });
      },
      subscribe: async () => () => {},
    };
    setEngineTransportForTests(transport);

    await useEngineStore.getState().ensureEngineRunning();

    expect(published.some((p) => p.kind === "start_engine" && p.origin === "engine")).toBe(true);
  });

  it("skips start when already running", async () => {
    const published: Array<{ origin: unknown; kind: string }> = [];
    setEngineTransportForTests({
      publish: async (origin, kind) => {
        published.push({ origin, kind });
      },
      subscribe: async () => () => {},
    });
    useEngineStore.setState({ status: makeStatus(true) });

    await useEngineStore.getState().ensureEngineRunning();

    expect(published).toHaveLength(0);
  });
});
