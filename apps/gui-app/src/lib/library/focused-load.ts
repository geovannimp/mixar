/** Load-time resolve of the focused library table row (MIDI LOAD). */

import type { LibraryTableRow } from "@/types";

export type FocusedLoadTarget =
  | { trackId: string; path?: undefined }
  | { path: string; trackId?: undefined };

type FocusedLoadResolver = () => FocusedLoadTarget | null;

let resolver: FocusedLoadResolver | null = null;

export function setFocusedLoadResolver(next: FocusedLoadResolver | null): void {
  resolver = next;
}

export function resolveFocusedLoad(): FocusedLoadTarget | null {
  return resolver?.() ?? null;
}

/** Map a visible table row to a load target (track id preferred over path). */
export function focusedLoadTargetFromRow(
  row: LibraryTableRow | undefined,
): FocusedLoadTarget | null {
  if (!row) {
    return null;
  }
  if (row.source === "library") {
    return { trackId: row.track.id };
  }
  if (row.libraryTrack) {
    return { trackId: row.libraryTrack.id };
  }
  return { path: row.file.path };
}
