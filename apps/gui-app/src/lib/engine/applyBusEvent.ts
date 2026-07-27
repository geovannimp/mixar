import { match } from "ts-pattern";
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

/** Merge engine deck snapshot onto UI deck; metadata/levels stay on `base` until unload. */
function mergeDeckSnapshot(existing: DeckStatus | undefined, snapshot: DeckSnapshot): DeckStatus {
  const base = existing ?? getDefaultDeck(snapshot.id);
  const unloaded = snapshot.duration_secs == null && existing?.duration_secs != null;
  return {
    ...base,
    id: snapshot.id,
    playing: snapshot.playing,
    volume: snapshot.volume,
    speed: snapshot.speed,
    eq: snapshot.eq,
    filter_db: snapshot.filter_db,
    gain_trim_db: snapshot.gain_trim_db,
    headphone_cue: snapshot.headphone_cue,
    sync_mode: snapshot.sync_mode,
    cue_point_secs: snapshot.cue_point_secs,
    quantize: snapshot.quantize,
    active_loop: snapshot.active_loop,
    is_master: base.is_master,
    position_secs: snapshot.position_secs ?? (unloaded ? null : base.position_secs),
    duration_secs: snapshot.duration_secs,
    levels: base.levels ?? ZERO_DECK_LEVELS,
    track: unloaded ? null : base.track,
    track_id: unloaded ? null : base.track_id,
    title: unloaded ? null : base.title,
    artist: unloaded ? null : base.artist,
    bpm: unloaded ? null : base.bpm,
    key: unloaded ? null : base.key,
    hot_cues: unloaded ? [] : base.hot_cues,
    saved_loops: unloaded ? [] : base.saved_loops,
    loudness_lufs: unloaded ? null : base.loudness_lufs,
    auto_gain_db: unloaded ? 0 : base.auto_gain_db,
  };
}

function mergeEngineStatusPayload(
  current: EngineStatus | null,
  payload: EngineStatusPayload,
): EngineStatus {
  const currentById = new Map(current?.decks.map((deck) => [deck.id, deck]));
  const masterDeck = payload.master_deck;
  return {
    running: payload.running,
    backend: current?.backend ?? "",
    sample_rate: payload.sample_rate,
    crossfader: payload.crossfader,
    cue_mix: payload.cue_mix,
    master_cue: payload.master_cue,
    master_deck: masterDeck,
    decks: payload.decks.map((snapshot) => {
      const merged = mergeDeckSnapshot(currentById.get(snapshot.id), snapshot);
      return { ...merged, is_master: merged.id === masterDeck };
    }),
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

  return match(body)
    .with({ type: "error" }, ({ message }) => ({
      status: current,
      revision: lastRevision,
      error: message,
    }))
    .with({ type: "notice" }, ({ message }) => ({
      status: current,
      revision: lastRevision,
      notice: message,
    }))
    .with({ type: "engine_status" }, ({ status }) => {
      if (wire.kind !== "status" || wire.revision < lastRevision) {
        return { status: current, revision: lastRevision };
      }
      return {
        status: mergeEngineStatusPayload(current, status),
        revision: wire.revision,
      };
    })
    .with({ type: "deck_updated" }, (deck) => {
      if (wire.kind !== "updated" || wire.revision < lastRevision) {
        return { status: current, revision: lastRevision };
      }
      if (!current) {
        return { status: null, revision: wire.revision };
      }
      const snapshot: DeckSnapshot = {
        id: deck.id,
        playing: deck.playing,
        volume: deck.volume,
        speed: deck.speed,
        eq: deck.eq,
        filter_db: deck.filter_db,
        gain_trim_db: deck.gain_trim_db,
        headphone_cue: deck.headphone_cue,
        sync_mode: deck.sync_mode,
        cue_point_secs: deck.cue_point_secs,
        quantize: deck.quantize,
        active_loop: deck.active_loop,
        position_secs: deck.position_secs,
        duration_secs: deck.duration_secs,
      };
      return {
        status: {
          ...current,
          decks: current.decks.map((d) => {
            if (d.id !== deck.id) {
              return d;
            }
            const merged = mergeDeckSnapshot(d, snapshot);
            return {
              ...merged,
              is_master: merged.id === (current.master_deck ?? 0),
            };
          }),
        },
        revision: wire.revision,
      };
    })
    .with({ type: "position" }, ({ position_secs }) => {
      const deckId = deckIdFromOrigin(wire.origin);
      if (wire.kind !== "position" || !current || deckId === null) {
        return { status: current, revision: lastRevision };
      }
      return {
        status: patchDeckPosition(current, deckId, position_secs),
        revision: lastRevision,
      };
    })
    .with({ type: "levels" }, (levels) => {
      const deckId = deckIdFromOrigin(wire.origin);
      if (wire.kind !== "levels" || !current || deckId === null) {
        return { status: current, revision: lastRevision };
      }
      return {
        status: patchDeckLevels(current, deckId, {
          peak_l: levels.peak_l,
          peak_r: levels.peak_r,
          peak_hold_l: levels.peak_hold_l,
          peak_hold_r: levels.peak_hold_r,
        }),
        revision: lastRevision,
      };
    })
    .with({ type: "empty" }, () => ({ status: current, revision: lastRevision }))
    .exhaustive();
}
