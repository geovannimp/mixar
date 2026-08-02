import type { TrackDragPayload } from "@/lib/libraryTable";
import type { LibraryTableRow } from "@/types";
import { engineActions } from "@/stores/engineStore";

export const TRACK_DRAG_TYPE = "track" as const;

export type TrackDragData = {
  type: typeof TRACK_DRAG_TYPE;
  payload: TrackDragPayload;
  row: LibraryTableRow;
};

export type DeckDropData = {
  type: "deck";
  deckId: number;
};

export type SamplerDropData = {
  type: "sampler";
  deckId: number;
  slot: number;
};

export type TrackDropData = DeckDropData | SamplerDropData;

export function deckDropId(deckId: number): string {
  return `deck:${deckId}`;
}

export function samplerDropId(deckId: number, slot: number): string {
  return `sampler:${deckId}:${slot}`;
}

export function trackDragId(rowKey: string): string {
  return `track:${rowKey}`;
}

export function isTrackDragData(data: unknown): data is TrackDragData {
  if (!data || typeof data !== "object") {
    return false;
  }
  return (data as TrackDragData).type === TRACK_DRAG_TYPE;
}

export function isTrackDropData(data: unknown): data is TrackDropData {
  if (!data || typeof data !== "object") {
    return false;
  }
  const type = (data as TrackDropData).type;
  return type === "deck" || type === "sampler";
}

/** Shared by dnd-kit onDragEnd and Tauri OS file drops. */
export function applyTrackDrop(target: TrackDropData, payload: TrackDragPayload): void {
  if (target.type === "sampler") {
    if (payload.trackId) {
      void engineActions.assignSamplerFromTrack(target.slot, payload.trackId, target.deckId);
      return;
    }
    void engineActions.assignSamplerFromPath(target.slot, payload.path, target.deckId);
    return;
  }

  if (payload.source === "library" && payload.trackId) {
    void engineActions.loadLibraryTrackToDeck(target.deckId, payload.trackId);
    return;
  }
  void engineActions.loadPathToDeck(target.deckId, payload.path);
}

export function trackPayloadFromOsPath(path: string): TrackDragPayload {
  const title = path.split(/[/\\]/).pop() ?? path;
  return {
    source: "filesystem",
    trackId: null,
    path,
    title,
  };
}

export const DROP_HIGHLIGHT_CLASS = "shadow-[inset_0_0_0_2px_rgba(52,211,153,0.55)]";
