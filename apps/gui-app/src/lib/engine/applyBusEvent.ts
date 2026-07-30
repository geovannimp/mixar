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
  const unloaded = snapshot.duration_ms == null && existing?.duration_ms != null;
  return {
    ...base,
    id: snapshot.id,
    track: unloaded ? null : snapshot.track,
    track_id: unloaded ? null : snapshot.track_id,
    title: unloaded ? null : snapshot.title,
    artist: unloaded ? null : snapshot.artist,
    bpm: unloaded ? null : snapshot.bpm,
    key: unloaded ? null : snapshot.key,
    playing: snapshot.playing,
    volume: snapshot.volume,
    speed: snapshot.speed,
    eq: snapshot.eq,
    filter_db: snapshot.filter_db,
    gain_trim_db: snapshot.gain_trim_db,
    headphone_cue: snapshot.headphone_cue,
    sync_mode: snapshot.sync_mode,
    cue_point_ms: snapshot.cue_point_ms,
    quantize: snapshot.quantize,
    active_loop: snapshot.active_loop,
    pad_mode: snapshot.pad_mode,
    is_master: base.is_master,
    position_ms: snapshot.position_ms ?? (unloaded ? null : base.position_ms),
    duration_ms: snapshot.duration_ms,
    levels: base.levels ?? ZERO_DECK_LEVELS,
    hot_cues: unloaded ? [] : snapshot.hot_cues,
    saved_loops: unloaded ? [] : snapshot.saved_loops,
    loudness_lufs: unloaded ? null : snapshot.loudness_lufs,
    auto_gain_db: unloaded ? 0 : snapshot.auto_gain_db,
    active_sampler_bank_id: unloaded ? null : snapshot.active_sampler_bank_id,
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
    sampler: payload.sampler ?? current?.sampler ?? DEFAULT_SAMPLER_STATUS,
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
        track: deck.track,
        track_id: deck.track_id,
        title: deck.title,
        artist: deck.artist,
        bpm: deck.bpm,
        key: deck.key,
        playing: deck.playing,
        volume: deck.volume,
        speed: deck.speed,
        eq: deck.eq,
        filter_db: deck.filter_db,
        gain_trim_db: deck.gain_trim_db,
        headphone_cue: deck.headphone_cue,
        sync_mode: deck.sync_mode,
        cue_point_ms: deck.cue_point_ms,
        quantize: deck.quantize,
        active_loop: deck.active_loop,
        pad_mode: deck.pad_mode,
        position_ms: deck.position_ms,
        duration_ms: deck.duration_ms,
        hot_cues: deck.hot_cues,
        saved_loops: deck.saved_loops,
        loudness_lufs: deck.loudness_lufs,
        auto_gain_db: deck.auto_gain_db,
        active_sampler_bank_id: deck.active_sampler_bank_id,
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
    .with({ type: "position" }, ({ position_ms }) => {
      const deckId = deckIdFromOrigin(wire.origin);
      if (wire.kind !== "position" || !current || deckId === null) {
        return { status: current, revision: lastRevision };
      }
      return {
        status: patchDeckPosition(current, deckId, position_ms),
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
