import { describe, expect, it } from "vitest";
import { encode } from "@msgpack/msgpack";
import { applyBusEvent } from "@/lib/engine/applyBusEvent";
import { getDefaultDeck } from "@/stores/defaultDeck";
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

function baseStatus(): EngineStatus {
  return {
    running: true,
    backend: "null",
    sample_rate: 48000,
    crossfader: 0.5,
    cue_mix: 0,
    master_cue: false,
    decks: [getDefaultDeck(0), getDefaultDeck(1)],
    sampler: DEFAULT_SAMPLER_STATUS,
  };
}

describe("applyBusEvent", () => {
  it("applies position updates for deck 0", () => {
    const current = baseStatus();
    current.decks[0] = { ...current.decks[0], playing: true, position_secs: 1 };
    const bytes = packWire({ deck: 0 }, "position", 1, {
      type: "position",
      position_secs: 12.25,
    });
    const patch = applyBusEvent(current, 1, bytes);
    expect(patch.status?.decks[0]?.position_secs).toBe(12.25);
  });

  it("deck_updated pause keeps position when provided", () => {
    const current = baseStatus();
    current.decks[0] = {
      ...current.decks[0],
      playing: true,
      position_secs: 12.25,
      duration_secs: 180,
    };
    const bytes = packWire({ deck: 0 }, "updated", 2, {
      type: "deck_updated",
      id: 0,
      playing: false,
      volume: 1,
      speed: 1,
      eq: { low: 0, mid: 0, high: 0 },
      position_secs: 12.25,
      duration_secs: 180,
    });
    const patch = applyBusEvent(current, 1, bytes);
    expect(patch.status?.decks[0]?.playing).toBe(false);
    expect(patch.status?.decks[0]?.position_secs).toBe(12.25);
  });

  it("deck_updated with null position keeps prior position", () => {
    const current = baseStatus();
    current.decks[0] = {
      ...current.decks[0],
      playing: true,
      position_secs: 12.25,
      duration_secs: 180,
    };
    const bytes = packWire({ deck: 0 }, "updated", 2, {
      type: "deck_updated",
      id: 0,
      playing: false,
      volume: 1,
      speed: 1,
      eq: { low: 0, mid: 0, high: 0 },
      position_secs: null,
      duration_secs: 180,
    });
    const patch = applyBusEvent(current, 1, bytes);
    expect(patch.status?.decks[0]?.position_secs).toBe(12.25);
  });
});
