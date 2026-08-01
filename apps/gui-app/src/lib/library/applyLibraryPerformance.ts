import type { DeckHotCueMarker, DeckSavedLoop, EngineStatus } from "@/types";
import {
  decodeEvtBody,
  decodeWire,
  type EvtBody,
  type WireHotCue,
  type WireSavedLoop,
} from "@/lib/library/wire";

function toHotCue(cue: WireHotCue): DeckHotCueMarker {
  return {
    slot: cue.slot,
    position_ms: cue.position_ms,
    loop_length_beats: cue.loop_length_beats ?? null,
    color: cue.color ?? null,
    label: cue.label ?? null,
  };
}

function toSavedLoop(loop: WireSavedLoop): DeckSavedLoop {
  return {
    slot: loop.slot,
    in_ms: loop.in_ms,
    out_ms: loop.out_ms,
    label: loop.label ?? null,
    color: loop.color ?? null,
  };
}

/** Apply library performance evt bytes onto decks that share the track id. */
export function applyLibraryPerformanceBytes(
  status: EngineStatus | null,
  bytes: Uint8Array,
): EngineStatus | null {
  if (!status) {
    return status;
  }
  let body: EvtBody;
  try {
    body = decodeEvtBody(decodeWire(bytes).body);
  } catch {
    return status;
  }
  switch (body.type) {
    case "hot_cues_changed": {
      const hotCues = body.hot_cues.map(toHotCue);
      return {
        ...status,
        decks: status.decks.map((deck) =>
          deck.track_id === body.track_id ? { ...deck, hot_cues: hotCues } : deck,
        ),
      };
    }
    case "loops_changed": {
      const savedLoops = body.loops.map(toSavedLoop);
      return {
        ...status,
        decks: status.decks.map((deck) =>
          deck.track_id === body.track_id ? { ...deck, saved_loops: savedLoops } : deck,
        ),
      };
    }
    case "empty":
    case "track_analyzed":
    case "error":
    case "notice":
      return status;
    default: {
      const _exhaustive: never = body;
      return _exhaustive;
    }
  }
}
