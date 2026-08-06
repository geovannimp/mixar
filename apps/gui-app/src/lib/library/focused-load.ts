/** Load-time resolve of the focused library table row (MIDI LOAD). */

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
