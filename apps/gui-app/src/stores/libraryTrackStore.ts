import { create } from "zustand";
import {
  decodeEvtBody,
  decodeWire,
  toTrackSummary,
  type WireHotCue,
  type WireSavedLoop,
  type WireTrackSummary,
} from "@/lib/library/wire";
import type { DeckHotCueMarker, DeckSavedLoop, TrackSummary } from "@/types";

export type LibraryTrack = {
  id: string;
  display_name: string;
  artist: string | null;
  title: string | null;
  album: string | null;
  genre: string | null;
  bpm: number | null;
  key: string | null;
  duration_ms: number | null;
  path: string;
  hot_cues: DeckHotCueMarker[];
  saved_loops: DeckSavedLoop[];
};

function emptyPerformance(): Pick<LibraryTrack, "hot_cues" | "saved_loops"> {
  return { hot_cues: [], saved_loops: [] };
}

function fromSummary(summary: TrackSummary | WireTrackSummary): LibraryTrack {
  const normalized: TrackSummary =
    typeof (summary as WireTrackSummary).display_name === "string" &&
    !("hot_cues" in (summary as object))
      ? toTrackSummary(summary as WireTrackSummary)
      : (summary as TrackSummary);
  const existing = useLibraryTrackStore.getState().tracks[normalized.id];
  return {
    id: normalized.id,
    display_name: normalized.display_name,
    artist: normalized.artist ?? null,
    title: normalized.title ?? null,
    album: normalized.album ?? null,
    genre: normalized.genre ?? null,
    bpm: normalized.bpm ?? null,
    key: normalized.key ?? null,
    duration_ms: normalized.duration_ms ?? null,
    path: normalized.path,
    hot_cues: existing?.hot_cues ?? [],
    saved_loops: existing?.saved_loops ?? [],
  };
}

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

type LibraryTrackStore = {
  tracks: Record<string, LibraryTrack>;
  upsertSummary: (summary: TrackSummary | WireTrackSummary) => void;
  upsertMany: (summaries: TrackSummary[]) => void;
  applyBusBytes: (bytes: Uint8Array) => void;
};

export const useLibraryTrackStore = create<LibraryTrackStore>((set, get) => ({
  tracks: {},
  upsertSummary: (summary) => {
    const track = fromSummary(summary);
    set((state) => ({
      tracks: { ...state.tracks, [track.id]: track },
    }));
  },
  upsertMany: (summaries) => {
    if (summaries.length === 0) {
      return;
    }
    set((state) => {
      const tracks = { ...state.tracks };
      for (const summary of summaries) {
        tracks[summary.id] = fromSummary(summary);
      }
      return { tracks };
    });
  },
  applyBusBytes: (bytes) => {
    let body;
    try {
      body = decodeEvtBody(decodeWire(bytes).body);
    } catch {
      return;
    }
    switch (body.type) {
      case "track_analyzed":
      case "track_updated": {
        get().upsertSummary(body.track);
        return;
      }
      case "hot_cues_changed": {
        const hotCues = body.hot_cues.map(toHotCue);
        set((state) => {
          const prev = state.tracks[body.track_id];
          const base = prev ?? {
            id: body.track_id,
            display_name: body.track_id,
            artist: null,
            title: null,
            album: null,
            genre: null,
            bpm: null,
            key: null,
            duration_ms: null,
            path: "",
            ...emptyPerformance(),
          };
          return {
            tracks: {
              ...state.tracks,
              [body.track_id]: { ...base, hot_cues: hotCues },
            },
          };
        });
        return;
      }
      case "loops_changed": {
        const savedLoops = body.loops.map(toSavedLoop);
        set((state) => {
          const prev = state.tracks[body.track_id];
          const base = prev ?? {
            id: body.track_id,
            display_name: body.track_id,
            artist: null,
            title: null,
            album: null,
            genre: null,
            bpm: null,
            key: null,
            duration_ms: null,
            path: "",
            ...emptyPerformance(),
          };
          return {
            tracks: {
              ...state.tracks,
              [body.track_id]: { ...base, saved_loops: savedLoops },
            },
          };
        });
        return;
      }
      case "empty":
      case "error":
      case "notice":
        return;
      default: {
        const _exhaustive: never = body;
        return _exhaustive;
      }
    }
  },
}));

export function getLibraryTrack(trackId: string | null | undefined): LibraryTrack | null {
  if (!trackId) {
    return null;
  }
  return useLibraryTrackStore.getState().tracks[trackId] ?? null;
}
