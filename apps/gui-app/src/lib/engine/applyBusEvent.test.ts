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

function deckUpdated(overrides: Record<string, unknown> = {}) {
  return {
    type: "deck_updated",
    id: 0,
    playing: false,
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
    pad_mode: "hot_cue",
    position_secs: null,
    duration_secs: null,
    ...overrides,
  };
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
    const bytes = packWire(
      { deck: 0 },
      "updated",
      2,
      deckUpdated({
        playing: false,
        position_secs: 12.25,
        duration_secs: 180,
      }),
    );
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
    const bytes = packWire(
      { deck: 0 },
      "updated",
      2,
      deckUpdated({
        playing: false,
        position_secs: null,
        duration_secs: 180,
      }),
    );
    const patch = applyBusEvent(current, 1, bytes);
    expect(patch.status?.decks[0]?.position_secs).toBe(12.25);
  });

  it("deck_updated applies channel-strip fields", () => {
    const current = baseStatus();
    const bytes = packWire(
      { deck: 0 },
      "updated",
      2,
      deckUpdated({
        filter_db: 3.5,
        gain_trim_db: -1.25,
        headphone_cue: true,
      }),
    );
    const patch = applyBusEvent(current, 1, bytes);
    expect(patch.status?.decks[0]?.filter_db).toBe(3.5);
    expect(patch.status?.decks[0]?.gain_trim_db).toBe(-1.25);
    expect(patch.status?.decks[0]?.headphone_cue).toBe(true);
  });

  it("deck_updated applies performance fields and clears metadata on unload", () => {
    const current = baseStatus();
    current.decks[0] = {
      ...current.decks[0],
      title: "Song",
      artist: "Artist",
      track: "/a.wav",
      duration_secs: 180,
      position_secs: 12,
    };
    const withLoop = packWire(
      { deck: 0 },
      "updated",
      2,
      deckUpdated({
        cue_point_secs: 1.5,
        quantize: false,
        active_loop: { in_secs: 0, out_secs: 2, active: true },
        duration_secs: 180,
        position_secs: 12,
      }),
    );
    const afterLoop = applyBusEvent(current, 1, withLoop);
    expect(afterLoop.status?.decks[0]?.cue_point_secs).toBe(1.5);
    expect(afterLoop.status?.decks[0]?.quantize).toBe(false);
    expect(afterLoop.status?.decks[0]?.active_loop).toEqual({
      in_secs: 0,
      out_secs: 2,
      active: true,
    });

    const unloaded = packWire(
      { deck: 0 },
      "updated",
      3,
      deckUpdated({
        duration_secs: null,
        position_secs: null,
      }),
    );
    const afterUnload = applyBusEvent(afterLoop.status, afterLoop.revision, unloaded);
    expect(afterUnload.status?.decks[0]?.title).toBeNull();
    expect(afterUnload.status?.decks[0]?.track).toBeNull();
    expect(afterUnload.status?.decks[0]?.duration_secs).toBeNull();
  });

  it("deck_updated applies sync_mode; status updates master_deck", () => {
    const current = baseStatus();
    const updated = packWire(
      { deck: 1 },
      "updated",
      2,
      deckUpdated({
        id: 1,
        speed: 1.2,
        sync_mode: "tempo",
      }),
    );
    const afterSync = applyBusEvent(current, 1, updated);
    expect(afterSync.status?.decks[1]?.sync_mode).toBe("tempo");
    expect(afterSync.status?.decks[1]?.speed).toBe(1.2);

    const statusBytes = packWire("mixer", "status", 3, {
      type: "engine_status",
      status: {
        running: true,
        sample_rate: 48000,
        crossfader: 0.5,
        cue_mix: 0,
        master_cue: false,
        master_deck: 1,
        decks: [deckUpdated({ id: 0 }), deckUpdated({ id: 1 })].map(
          ({ type: _t, ...snap }) => snap,
        ),
      },
    });
    const afterMaster = applyBusEvent(afterSync.status, afterSync.revision, statusBytes);
    expect(afterMaster.status?.master_deck).toBe(1);
    expect(afterMaster.status?.decks[1]?.is_master).toBe(true);
    expect(afterMaster.status?.decks[0]?.is_master).toBe(false);
  });
});
