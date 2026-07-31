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
    track: null,
    track_id: null,
    title: null,
    artist: null,
    bpm: null,
    key: null,
    playing: false,
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
    position_ms: null,
    duration_ms: null,
    hot_cues: [],
    saved_loops: [],
    loudness_lufs: null,
    auto_gain_db: 0,
    active_sampler_bank_id: null,
    top_jog_mode: "vinyl",
    outer_jog_mode: "pitch_bend",
    jog_touching: false,
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
    current.decks[0] = { ...current.decks[0], playing: true, position_ms: 1000 };
    const bytes = packWire({ deck: 0 }, "position", 1, {
      type: "position",
      position_ms: 12250,
    });
    const patch = applyBusEvent(current, 1, bytes);
    expect(patch.status?.decks[0]?.position_ms).toBe(12250);
  });

  it("deck_updated pause keeps position when provided", () => {
    const current = baseStatus();
    current.decks[0] = {
      ...current.decks[0],
      playing: true,
      position_ms: 12250,
      duration_ms: 180000,
    };
    const bytes = packWire(
      { deck: 0 },
      "updated",
      2,
      deckUpdated({
        playing: false,
        position_ms: 12250,
        duration_ms: 180000,
      }),
    );
    const patch = applyBusEvent(current, 1, bytes);
    expect(patch.status?.decks[0]?.playing).toBe(false);
    expect(patch.status?.decks[0]?.position_ms).toBe(12250);
  });

  it("deck_updated with null position keeps prior position", () => {
    const current = baseStatus();
    current.decks[0] = {
      ...current.decks[0],
      playing: true,
      position_ms: 12250,
      duration_ms: 180000,
    };
    const bytes = packWire(
      { deck: 0 },
      "updated",
      2,
      deckUpdated({
        playing: false,
        position_ms: null,
        duration_ms: 180000,
      }),
    );
    const patch = applyBusEvent(current, 1, bytes);
    expect(patch.status?.decks[0]?.position_ms).toBe(12250);
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

  it("deck_updated applies library-backed deck fields from the bus", () => {
    const current = baseStatus();
    const bytes = packWire(
      { deck: 0 },
      "updated",
      2,
      deckUpdated({
        track: "/music/song.wav",
        track_id: "track-1",
        title: "Song",
        artist: "Artist",
        bpm: 128,
        key: "8A",
        hot_cues: [
          { slot: 1, position_ms: 12500, loop_length_beats: null, color: null, label: null },
        ],
        saved_loops: [{ slot: 2, in_ms: 4000, out_ms: 8000, label: null, color: null }],
        loudness_lufs: -8.2,
        auto_gain_db: 1.5,
        active_sampler_bank_id: "bank-1",
        duration_ms: 180000,
        position_ms: 12500,
      }),
    );

    const patch = applyBusEvent(current, 1, bytes);
    expect(patch.status?.decks[0]).toMatchObject({
      track: "/music/song.wav",
      track_id: "track-1",
      title: "Song",
      artist: "Artist",
      bpm: 128,
      key: "8A",
      loudness_lufs: -8.2,
      auto_gain_db: 1.5,
      active_sampler_bank_id: "bank-1",
    });
    expect(patch.status?.decks[0]?.hot_cues).toEqual([
      { slot: 1, position_ms: 12500, loop_length_beats: null, color: null, label: null },
    ]);
    expect(patch.status?.decks[0]?.saved_loops).toEqual([
      { slot: 2, in_ms: 4000, out_ms: 8000, label: null, color: null },
    ]);
  });

  it("deck_updated from engine keeps host track metadata (jog touch)", () => {
    const current = baseStatus();
    current.decks[0] = {
      ...current.decks[0],
      track: "/music/song.wav",
      track_id: "track-1",
      title: "Song",
      artist: "Artist",
      bpm: 128,
      duration_ms: 180000,
      position_ms: 12000,
      hot_cues: [
        { slot: 1, position_ms: 12500, loop_length_beats: null, color: null, label: null },
      ],
      jog_touching: false,
    };
    const bytes = packWire(
      { deck: 0 },
      "updated",
      2,
      deckUpdated({
        playing: true,
        duration_ms: 180000,
        position_ms: 12100,
        jog_touching: true,
        // Engine snapshots omit library fields:
        track: null,
        track_id: null,
        title: null,
        artist: null,
        bpm: null,
        hot_cues: [],
      }),
    );
    const patch = applyBusEvent(current, 1, bytes);
    expect(patch.status?.decks[0]).toMatchObject({
      track: "/music/song.wav",
      track_id: "track-1",
      title: "Song",
      artist: "Artist",
      bpm: 128,
      jog_touching: true,
      playing: true,
      position_ms: 12100,
    });
    expect(patch.status?.decks[0]?.hot_cues).toEqual([
      { slot: 1, position_ms: 12500, loop_length_beats: null, color: null, label: null },
    ]);
  });

  it("deck_updated applies performance fields and clears metadata on unload", () => {
    const current = baseStatus();
    current.decks[0] = {
      ...current.decks[0],
      title: "Song",
      artist: "Artist",
      track: "/a.wav",
      duration_ms: 180000,
      position_ms: 12000,
    };
    const withLoop = packWire(
      { deck: 0 },
      "updated",
      2,
      deckUpdated({
        cue_point_ms: 1500,
        quantize: false,
        active_loop: { in_ms: 0, out_ms: 2000, active: true },
        duration_ms: 180000,
        position_ms: 12000,
      }),
    );
    const afterLoop = applyBusEvent(current, 1, withLoop);
    expect(afterLoop.status?.decks[0]?.cue_point_ms).toBe(1500);
    expect(afterLoop.status?.decks[0]?.quantize).toBe(false);
    expect(afterLoop.status?.decks[0]?.active_loop).toEqual({
      in_ms: 0,
      out_ms: 2000,
      active: true,
    });

    const unloaded = packWire(
      { deck: 0 },
      "updated",
      3,
      deckUpdated({
        duration_ms: null,
        position_ms: null,
      }),
    );
    const afterUnload = applyBusEvent(afterLoop.status, afterLoop.revision, unloaded);
    expect(afterUnload.status?.decks[0]?.title).toBeNull();
    expect(afterUnload.status?.decks[0]?.track).toBeNull();
    expect(afterUnload.status?.decks[0]?.duration_ms).toBeNull();
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
        sampler: DEFAULT_SAMPLER_STATUS,
      },
    });
    const afterMaster = applyBusEvent(afterSync.status, afterSync.revision, statusBytes);
    expect(afterMaster.status?.master_deck).toBe(1);
    expect(afterMaster.status?.decks[1]?.is_master).toBe(true);
    expect(afterMaster.status?.decks[0]?.is_master).toBe(false);
  });

  it("status updates sampler state from the bus", () => {
    const current = baseStatus();
    const statusBytes = packWire("mixer", "status", 2, {
      type: "engine_status",
      status: {
        running: true,
        sample_rate: 48000,
        crossfader: 0.25,
        cue_mix: 0.1,
        master_cue: true,
        master_deck: 0,
        decks: [deckUpdated({ id: 0 }), deckUpdated({ id: 1 })].map(
          ({ type: _t, ...snap }) => snap,
        ),
        sampler: {
          banks: [{ id: "bank-1", name: "Main", play_mode: "oneshot", sort_index: 0 }],
          active_bank_id: "bank-1",
          active_bank_name: "Main",
          bank_play_mode: "oneshot",
          deck_slots: [[], []],
          effective_play_modes: ["oneshot", "oneshot"],
        },
      },
    });

    const patch = applyBusEvent(current, 1, statusBytes);
    expect(patch.status?.sampler.active_bank_id).toBe("bank-1");
    expect(patch.status?.sampler.banks).toEqual([
      { id: "bank-1", name: "Main", play_mode: "oneshot", sort_index: 0 },
    ]);
  });
});
