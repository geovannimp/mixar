import { getDefaultDeck } from "@/stores/defaultDeck";
import { DEFAULT_SAMPLER_STATUS } from "@/stores/defaultSampler";
import { ZERO_DECK_LEVELS, type DeckStatus, type EngineStatus } from "@/types";
import {
  decodeEvtBody,
  decodeWire,
  type DeckSnapshot,
  type EngineStatusPayload,
  type Origin,
} from "@/lib/engine/wire";
import { patchDeckLevels, patchDeckPosition } from "@/lib/engineEvents";

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

/** Merge engine deck snapshot onto UI deck; metadata/levels stay on `base`. */
function mergeDeckSnapshot(existing: DeckStatus | undefined, snapshot: DeckSnapshot): DeckStatus {
  const base = existing ?? getDefaultDeck(snapshot.id);
  return {
    ...base,
    id: snapshot.id,
    playing: snapshot.playing,
    volume: snapshot.volume,
    speed: snapshot.speed,
    eq: snapshot.eq,
    position_secs: snapshot.position_secs,
    duration_secs: snapshot.duration_secs,
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
    decks: payload.decks.map((snapshot) =>
      mergeDeckSnapshot(currentById.get(snapshot.id), snapshot),
    ),
    sampler: current?.sampler ?? DEFAULT_SAMPLER_STATUS,
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
    const snapshot: DeckSnapshot = {
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
          deck.id === body.id ? mergeDeckSnapshot(deck, snapshot) : deck,
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
