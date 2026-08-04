export interface DeckHotCueMarker {
  slot: number;
  position_ms: number;
  loop_length_beats?: number | null;
  color?: string | null;
  label?: string | null;
}

export interface DeckSavedLoop {
  slot: number;
  in_ms: number;
  out_ms: number;
  label?: string | null;
  color?: string | null;
}

export interface DeckActiveLoop {
  in_ms: number;
  out_ms: number;
  active: boolean;
}

export interface DeckLoopMarker {
  start_ms: number;
  end_ms: number;
  active?: boolean;
}

export interface DeckEq {
  low: number;
  mid: number;
  high: number;
}

export const DEFAULT_DECK_EQ: DeckEq = { low: 0, mid: 0, high: 0 };

export type LevelMeterMode = "mono" | "stereo";

export interface DeckLevels {
  peak_l: number;
  peak_r: number;
  peak_hold_l: number;
  peak_hold_r: number;
}

export const ZERO_DECK_LEVELS: DeckLevels = {
  peak_l: 0,
  peak_r: 0,
  peak_hold_l: 0,
  peak_hold_r: 0,
};

export type SyncMode = "off" | "tempo" | "beat";
export type PadMode = "hot_cue" | "loop_roll" | "beat_jump" | "sampler";
export type JogMode = "vinyl" | "pitch_bend" | "ignore";
export type KeyDisplayMode = "musical" | "camelot";
export type SamplerPlayMode = "oneshot" | "hold" | "loop";
export type SamplerStripRoute = "before" | "after";

export interface DeckStatus {
  id: number;
  track: string | null;
  track_id: string | null;
  title: string | null;
  artist: string | null;
  bpm: number | null;
  key: string | null;
  playing: boolean;
  volume: number;
  speed: number;
  eq: DeckEq;
  position_ms: number | null;
  duration_ms: number | null;
  cue_point_ms: number | null;
  quantize: boolean;
  hot_cues: DeckHotCueMarker[];
  saved_loops: DeckSavedLoop[];
  active_loop: DeckActiveLoop | null;
  filter_db: number;
  gain_trim_db: number;
  loudness_lufs: number | null;
  auto_gain_db: number;
  sync_mode: SyncMode;
  is_master: boolean;
  pad_mode: PadMode;
  headphone_cue: boolean;
  active_sampler_bank_id: string | null;
  top_jog_mode: JogMode;
  outer_jog_mode: JogMode;
  jog_touching: boolean;
  levels: DeckLevels;
}

export interface WaveformFrame {
  width: number;
  height: number;
  rgba_base64: string;
  center_ms: number;
  cover_start_ms: number;
  cover_end_ms: number;
  visible_ms: number;
}

export interface SamplerSlotInfo {
  label: string | null;
  track_id: string | null;
  path: string | null;
  duration_ms: number | null;
}

export interface SamplerBankInfo {
  id: string;
  name: string;
  play_mode: SamplerPlayMode | null;
  sort_index: number;
}

export interface SamplerStatus {
  banks: SamplerBankInfo[];
  active_bank_id: string | null;
  active_bank_name: string | null;
  bank_play_mode: SamplerPlayMode | null;
  deck_slots: SamplerSlotInfo[][];
  effective_play_modes: SamplerPlayMode[];
}

export interface EngineStatus {
  running: boolean;
  backend: string;
  sample_rate: number;
  crossfader: number;
  cue_mix: number;
  master_cue: boolean;
  master_deck?: number;
  decks: DeckStatus[];
  sampler: SamplerStatus;
}

export type BusChannelMode = "stereo" | "mono";

export interface BusRouteSettings {
  device_id: string;
  left_channel: number;
  right_channel: number;
  /** Defaults to stereo when omitted (older settings). */
  mode?: BusChannelMode;
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
  library_table_columns: LibraryTableColumn[];
  volume_normalizer_enabled: boolean;
  target_lufs: number;
  sampler_play_mode: SamplerPlayMode;
  sampler_strip_route: SamplerStripRoute;
  deck_default_sampler_bank_id: [string | null, string | null];
  default_top_jog_mode: JogMode;
  default_outer_jog_mode: JogMode;
}

export type LibraryTableColumn =
  | "title"
  | "artist"
  | "album"
  | "genre"
  | "bpm"
  | "key"
  | "duration"
  | "path";

export type LibraryTableRow =
  | { source: "library"; track: TrackSummary }
  | { source: "filesystem"; file: FsEntry; libraryTrack?: TrackSummary };

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
  duration_ms: number | null;
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

export interface ResolvedLibraryTrack {
  request_path: string;
  track: TrackSummary;
}

export type SettingsSection = "audio" | "library" | "controllers";

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
