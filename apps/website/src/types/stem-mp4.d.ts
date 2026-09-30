// `stem-mp4` ships no type declarations (no `types` field; its `exports`
// subpaths map straight to JS). Declare the subpath the site imports so the
// website typecheck can run. Shapes read from the implementation in
// node_modules/stem-mp4/src/extractor.js.
declare module "stem-mp4/extractor" {
  export type ExtractedTrack = Uint8Array;

  export interface TrackInfo {
    index: number;
    size: number;
    codec: string;
  }

  /** Rebuilds one track as a standalone M4A file. */
  export function extractTrack(data: Uint8Array, trackIndex: number): ExtractedTrack;

  /** Throws if the `moov` atom is missing. */
  export function extractAllTracks(data: Uint8Array): ExtractedTrack[];

  export function getTrackCount(data: Uint8Array): number;

  export function getTrackInfo(data: Uint8Array): TrackInfo[];
}
