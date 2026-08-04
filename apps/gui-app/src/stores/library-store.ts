import { create, type StateCreator, type StoreMutatorIdentifier } from "zustand";
import { getLibraryTransport } from "@/lib/library/transport";
import {
  decodeEvtBody,
  decodeWire,
  toTrackSummary,
  type WireHotCue,
  type WireSavedLoop,
  type WireTrackSummary,
} from "@/lib/library/wire";
import type {
  AddFolderCollectionResult,
  CollectionSummary,
  DeckHotCueMarker,
  DeckSavedLoop,
  TrackSummary,
} from "@/types";

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

export type LibraryCollection = CollectionSummary & {
  trackIds: string[];
};

function stubTrack(trackId: string): LibraryTrack {
  return {
    id: trackId,
    display_name: trackId,
    artist: null,
    title: null,
    album: null,
    genre: null,
    bpm: null,
    key: null,
    duration_ms: null,
    path: "",
    hot_cues: [],
    saved_loops: [],
  };
}

function trackFromSummary(
  summary: TrackSummary | WireTrackSummary,
  existing: LibraryTrack | undefined,
): LibraryTrack {
  const normalized = toTrackSummary(summary as WireTrackSummary);
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

function navigateIndex(current: number, count: number, delta: 1 | -1): number {
  if (count <= 0) {
    return 0;
  }
  return Math.min(count - 1, Math.max(0, current + delta));
}

export function toTrackSummaryView(track: LibraryTrack): TrackSummary {
  return {
    id: track.id,
    display_name: track.display_name,
    artist: track.artist,
    title: track.title,
    album: track.album,
    genre: track.genre,
    bpm: track.bpm,
    key: track.key,
    duration_ms: track.duration_ms,
    path: track.path,
  };
}

type LibraryState = {
  collections: Record<string, LibraryCollection>;
  collectionIds: string[];
  tracks: Record<string, LibraryTrack>;
  error: string | null;
  busy: boolean;
  analyzingTrackId: string | null;
  /** Last track id that finished analyze (for UI side effects). */
  lastAnalyzedTrackId: string | null;
  /** Focused row in the visible track table (MIDI library nav). */
  focusedTrackRowIndex: number;
  /** Visible row count for clamp; panel updates when table rows change. */
  trackFocusRowCount: number;

  setTrackFocusRowCount: (count: number) => void;
  navigateTrackFocus: (delta: 1 | -1) => void;

  refreshCollections: () => Promise<void>;
  loadCollectionTracks: (collectionId: string) => Promise<void>;
  addFolderCollectionFromPath: (folderPath: string) => Promise<AddFolderCollectionResult | null>;
  analyzeTrack: (trackId: string) => Promise<void>;
};

type LibrarySet = (
  partial: LibraryState | Partial<LibraryState> | ((state: LibraryState) => Partial<LibraryState>),
) => void;

function applyBusBytes(set: LibrarySet, bytes: Uint8Array): void {
  let wire;
  let body;
  try {
    wire = decodeWire(bytes);
    body = decodeEvtBody(wire.body);
  } catch {
    return;
  }

  if (wire.origin === "library_navigation") {
    if (wire.kind === "navigate_next") {
      set((state) => ({
        focusedTrackRowIndex: navigateIndex(
          state.focusedTrackRowIndex,
          state.trackFocusRowCount,
          1,
        ),
      }));
      return;
    }
    if (wire.kind === "navigate_prev") {
      set((state) => ({
        focusedTrackRowIndex: navigateIndex(
          state.focusedTrackRowIndex,
          state.trackFocusRowCount,
          -1,
        ),
      }));
      return;
    }
    return;
  }

  switch (body.type) {
    case "track_analyzed":
    case "track_updated": {
      const summary = toTrackSummary(body.track);
      set((state) => {
        const existing = state.tracks[summary.id];
        const track = trackFromSummary(summary, existing);
        const next: Partial<LibraryState> = {
          tracks: { ...state.tracks, [track.id]: track },
        };
        if (body.type === "track_analyzed") {
          next.analyzingTrackId =
            state.analyzingTrackId === track.id ? null : state.analyzingTrackId;
          next.lastAnalyzedTrackId = track.id;
        }
        return next;
      });
      return;
    }
    case "hot_cues_changed": {
      const hotCues = body.hot_cues.map(toHotCue);
      set((state) => {
        const base = state.tracks[body.track_id] ?? stubTrack(body.track_id);
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
        const base = state.tracks[body.track_id] ?? stubTrack(body.track_id);
        return {
          tracks: {
            ...state.tracks,
            [body.track_id]: { ...base, saved_loops: savedLoops },
          },
        };
      });
      return;
    }
    case "error": {
      set((state) => ({
        error: body.message,
        analyzingTrackId: body.track_id
          ? state.analyzingTrackId === body.track_id
            ? null
            : state.analyzingTrackId
          : null,
      }));
      return;
    }
    case "notice":
    case "empty":
      return;
    default: {
      const _exhaustive: never = body;
      return _exhaustive;
    }
  }
}

type LibraryBus = <
  T,
  Mps extends [StoreMutatorIdentifier, unknown][] = [],
  Mcs extends [StoreMutatorIdentifier, unknown][] = [],
>(
  f: StateCreator<T, Mps, Mcs>,
) => StateCreator<T, Mps, Mcs>;

type LibraryBusImpl = <T>(f: StateCreator<T, [], []>) => StateCreator<T, [], []>;

/** Subscribe once to `library://bus` and apply events via `set` (not store actions). */
const libraryBusImpl: LibraryBusImpl = (f) => (set, get, store) => {
  let started = false;
  const start = () => {
    if (started) {
      return;
    }
    started = true;
    void getLibraryTransport().subscribe((bytes) => {
      applyBusBytes(set as LibrarySet, bytes);
    });
  };
  queueMicrotask(start);
  return f(set, get, store);
};

export const libraryBus = libraryBusImpl as unknown as LibraryBus;

/** Test helper: apply a bus evt without going through the transport. */
export function applyLibraryBusBytesForTests(bytes: Uint8Array): void {
  applyBusBytes(useLibraryStore.setState as LibrarySet, bytes);
}

const transport = getLibraryTransport();

export const useLibraryStore = create<LibraryState>()(
  libraryBus((set, get) => ({
    collections: {},
    collectionIds: [],
    tracks: {},
    error: null,
    busy: false,
    analyzingTrackId: null,
    lastAnalyzedTrackId: null,
    focusedTrackRowIndex: 0,
    trackFocusRowCount: 0,

    setTrackFocusRowCount: (count) => {
      set((state) => ({
        trackFocusRowCount: Math.max(0, count),
        focusedTrackRowIndex:
          count <= 0 ? 0 : Math.min(state.focusedTrackRowIndex, Math.max(0, count - 1)),
      }));
    },

    navigateTrackFocus: (delta) => {
      set((state) => ({
        focusedTrackRowIndex: navigateIndex(
          state.focusedTrackRowIndex,
          state.trackFocusRowCount,
          delta,
        ),
      }));
    },

    refreshCollections: async () => {
      const next = await transport.listCollections();
      set((state) => {
        const collections: Record<string, LibraryCollection> = {};
        for (const summary of next) {
          const prev = state.collections[summary.id];
          collections[summary.id] = {
            ...summary,
            trackIds: prev?.trackIds ?? [],
          };
        }
        return {
          collections,
          collectionIds: next.map((collection) => collection.id),
        };
      });
    },

    loadCollectionTracks: async (collectionId) => {
      const summaries = await transport.listCollectionTracks(collectionId);
      set((state) => {
        const tracks = { ...state.tracks };
        const trackIds: string[] = [];
        for (const summary of summaries) {
          tracks[summary.id] = trackFromSummary(summary, tracks[summary.id]);
          trackIds.push(summary.id);
        }
        const collection = state.collections[collectionId];
        if (!collection) {
          return { tracks };
        }
        return {
          tracks,
          collections: {
            ...state.collections,
            [collectionId]: {
              ...collection,
              trackIds,
              track_count: trackIds.length,
            },
          },
        };
      });
    },

    addFolderCollectionFromPath: async (folderPath) => {
      set({ busy: true, error: null });
      try {
        const result = await transport.addFolderCollection(folderPath);
        await get().refreshCollections();
        await get().loadCollectionTracks(result.collection.id);
        return result;
      } catch (err) {
        set({ error: String(err) });
        return null;
      } finally {
        set({ busy: false });
      }
    },

    analyzeTrack: async (trackId) => {
      set({ analyzingTrackId: trackId, error: null, lastAnalyzedTrackId: null });
      try {
        await transport.publish("library", "analyze_track", {
          track_id: trackId,
          force: false,
        });
      } catch (err) {
        set({ error: String(err), analyzingTrackId: null });
      }
    },
  })),
);
