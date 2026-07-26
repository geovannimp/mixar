import { getDefaultDeck } from "@/stores/defaultDeck";
import { ZERO_DECK_LEVELS, type DeckStatus, type EngineStatus, type SamplerStatus } from "@/types";
import {
  decodeEvtBody,
  decodeWire,
  type DeckSnapshot,
  type EngineStatusPayload,
  type Origin,
} from "@/lib/engine/wire";
import { patchDeckLevels, patchDeckPosition } from "@/lib/engineEvents";

const EMPTY_SAMPLER_STATUS: SamplerStatus = {
  banks: [],
  active_bank_id: null,
  active_bank_name: null,
  bank_play_mode: null,
  deck_slots: [
    [
      { label: null, track_id: null, path: null, duration_secs: null },
      { label: null, track_id: null, path: null, duration_secs: null },
      { label: null, track_id: null, path: null, duration_secs: null },
      { label: null, track_id: null, path: null, duration_secs: null },
      { label: null, track_id: null, path: null, duration_secs: null },
      { label: null, track_id: null, path: null, duration_secs: null },
      { label: null, track_id: null, path: null, duration_secs: null },
      { label: null, track_id: null, path: null, duration_secs: null },
    ],
    [
      { label: null, track_id: null, path: null, duration_secs: null },
      { label: null, track_id: null, path: null, duration_secs: null },
      { label: null, track_id: null, path: null, duration_secs: null },
      { label: null, track_id: null, path: null, duration_secs: null },
      { label: null, track_id: null, path: null, duration_secs: null },
      { label: null, track_id: null, path: null, duration_secs: null },
      { label: null, track_id: null, path: null, duration_secs: null },
      { label: null, track_id: null, path: null, duration_secs: null },
    ],
  ],
  effective_play_modes: ["oneshot", "oneshot"],
};

export type BusEventPatch = {
  status: EngineStatus | null;
  revision: number;
  error?: string;
  notice?: string;
};

function deckIdFromOrigin(origin: Origin): number | null {
  if (typeof origin === "object" && "deck" in origin) {
    return origin.deck;
  }
  return null;
}

function mergeSlimDeck(existing: DeckStatus | undefined, slim: DeckSnapshot): DeckStatus {
  const base = existing ?? getDefaultDeck(slim.id);
  return {
    ...base,
    id: slim.id,
    playing: slim.playing,
    volume: slim.volume,
    speed: slim.speed,
    eq: slim.eq,
    position_secs: slim.position_secs,
    duration_secs: slim.duration_secs,
    levels: base.levels ?? ZERO_DECK_LEVELS,
    title: base.title,
    artist: base.artist,
    hot_cues: base.hot_cues,
  };
}

function mergeEngineStatusPayload(
  current: EngineStatus | null,
  payload: EngineStatusPayload,
): EngineStatus {
  const currentById = new Map(current?.decks.map((deck) => [deck.id, deck]));
  return {
    running: payload.running,
    backend: current?.backend ?? "",
    sample_rate: payload.sample_rate,
    crossfader: payload.crossfader,
    cue_mix: payload.cue_mix,
    master_cue: payload.master_cue,
    master_deck: current?.master_deck,
    decks: payload.decks.map((slim) => mergeSlimDeck(currentById.get(slim.id), slim)),
    sampler: current?.sampler ?? EMPTY_SAMPLER_STATUS,
  };
}

export function applyBusEvent(
  current: EngineStatus | null,
  lastRevision: number,
  bytes: Uint8Array,
): BusEventPatch {
  const wire = decodeWire(bytes);
  const body = decodeEvtBody(wire.body);

  if (body.type === "error") {
    return { status: current, revision: lastRevision, error: body.message };
  }

  if (body.type === "notice") {
    return { status: current, revision: lastRevision, notice: body.message };
  }

  if (wire.kind === "status" && body.type === "engine_status") {
    if (wire.revision < lastRevision) {
      return { status: current, revision: lastRevision };
    }
    return {
      status: mergeEngineStatusPayload(current, body.status),
      revision: wire.revision,
    };
  }

  if (wire.kind === "updated" && body.type === "deck_updated") {
    if (wire.revision < lastRevision) {
      return { status: current, revision: lastRevision };
    }
    if (!current) {
      return { status: null, revision: wire.revision };
    }
    const slim: DeckSnapshot = {
      id: body.id,
      playing: body.playing,
      volume: body.volume,
      speed: body.speed,
      eq: body.eq,
      position_secs: body.position_secs,
      duration_secs: body.duration_secs,
    };
    return {
      status: {
        ...current,
        decks: current.decks.map((deck) =>
          deck.id === body.id ? mergeSlimDeck(deck, slim) : deck,
        ),
      },
      revision: wire.revision,
    };
  }

  if (wire.kind === "position" && body.type === "position") {
    const deckId = deckIdFromOrigin(wire.origin);
    if (!current || deckId === null) {
      return { status: current, revision: lastRevision };
    }
    return {
      status: patchDeckPosition(current, deckId, body.position_secs),
      revision: lastRevision,
    };
  }

  if (wire.kind === "levels" && body.type === "levels") {
    const deckId = deckIdFromOrigin(wire.origin);
    if (!current || deckId === null) {
      return { status: current, revision: lastRevision };
    }
    return {
      status: patchDeckLevels(current, deckId, {
        peak_l: body.peak_l,
        peak_r: body.peak_r,
        peak_hold_l: body.peak_hold_l,
        peak_hold_r: body.peak_hold_r,
      }),
      revision: lastRevision,
    };
  }

  return { status: current, revision: lastRevision };
}
