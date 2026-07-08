export interface DeckEq {
  low: number;
  mid: number;
  high: number;
}

export const DEFAULT_DECK_EQ: DeckEq = { low: 0, mid: 0, high: 0 };

export interface DeckStatus {
  id: number;
  track: string | null;
  track_id: string | null;
  playing: boolean;
  volume: number;
  eq: DeckEq;
  position_secs: number | null;
  duration_secs: number | null;
}

export interface WaveformFrame {
  width: number;
  height: number;
  rgba_base64: string;
  center_secs: number;
  cover_start_secs: number;
  cover_end_secs: number;
  visible_secs: number;
}

export interface EngineStatus {
  running: boolean;
  backend: string;
  sample_rate: number;
  crossfader: number;
  decks: DeckStatus[];
}

export interface BusRouteSettings {
  device_id: string;
  left_channel: number;
  right_channel: number;
}

export interface AudioDeviceSummary {
  id: string;
  name: string;
  is_default: boolean;
}

export type AnalysisMode = "fast" | "precise" | "complete";

export type ResamplerQuality = "low" | "medium" | "high";

export interface AppSettings {
  backend: string;
  sample_rate: number;
  buffer_size: number;
  low_latency: boolean;
  resampler_quality: ResamplerQuality;
  master_bus: BusRouteSettings;
  preview_enabled: boolean;
  preview_bus: BusRouteSettings;
  analysis_duration: AnalysisMode;
  scan_folder_tree: boolean;
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

export type SettingsSection = "audio" | "library";

export type LibrarySourceTab = "collections" | "drive";

export interface VolumeInfo {
  name: string;
  path: string;
  is_removable: boolean;
}

export interface FsEntry {
  name: string;
  path: string;
}

export interface DirectoryListing {
  path: string;
  parent: string | null;
  directories: FsEntry[];
  audio_files: FsEntry[];
}
