export interface DeckStatus {
  id: number;
  track: string | null;
  playing: boolean;
}

export interface EngineStatus {
  running: boolean;
  backend: string;
  sample_rate: number;
  decks: DeckStatus[];
}

export interface CollectionSummary {
  id: string;
  name: string;
  kind: string;
  path: string | null;
  track_count: number;
}

export interface TrackSummary {
  id: string;
  display_name: string;
  artist: string | null;
  title: string | null;
  album: string | null;
  genre: string | null;
  bpm: number | null;
  key: string | null;
  duration_secs: number | null;
  path: string;
}

export interface ScanReport {
  added: number;
  updated: number;
  skipped: number;
  failed: number;
  errors: string[];
}

export interface AddFolderCollectionResult {
  collection: CollectionSummary;
  scan: ScanReport;
}
