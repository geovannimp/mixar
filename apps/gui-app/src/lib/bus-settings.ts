import type {
  AppSettings,
  BusChannelMode,
  BusRouteSettings,
  JogMode,
  SamplerPlayMode,
  SamplerStripRoute,
} from "@/types";
import { DEFAULT_LIBRARY_TABLE_COLUMNS, normalizeLibraryTableColumns } from "./library-table";

export const DEFAULT_DEVICE_ID = "default";
export const DEFAULT_VOLUME_NORMALIZER_ENABLED = true;
export const DEFAULT_TARGET_LUFS = -18;
export const DEFAULT_SAMPLER_PLAY_MODE: SamplerPlayMode = "oneshot";
export const DEFAULT_SAMPLER_STRIP_ROUTE: SamplerStripRoute = "before";
export const DEFAULT_TOP_JOG_MODE: JogMode = "vinyl";
export const DEFAULT_OUTER_JOG_MODE: JogMode = "pitch_bend";
/** Default tempo fader half-span as pitch fraction (`0.06` = ±6%). */
export const DEFAULT_TEMPO_RANGE = 0.06;
/** Pioneer / Mixxx DDJ-400 cycle steps. */
export const TEMPO_RANGE_STEPS = [0.06, 0.1, 0.16, 0.25] as const;
export const MIN_TARGET_LUFS = -24;
export const MAX_TARGET_LUFS = -9;

export const DEFAULT_MASTER_BUS: BusRouteSettings = {
  device_id: DEFAULT_DEVICE_ID,
  left_channel: 1,
  right_channel: 2,
  mode: "stereo",
};

export const DEFAULT_PREVIEW_BUS: BusRouteSettings = {
  device_id: DEFAULT_DEVICE_ID,
  left_channel: 3,
  right_channel: 4,
  mode: "stereo",
};

function normalizeBusRoute(route: BusRouteSettings): BusRouteSettings {
  const mode: BusChannelMode = route.mode === "mono" ? "mono" : "stereo";
  return {
    device_id: route.device_id,
    left_channel: route.left_channel,
    right_channel: route.right_channel,
    mode,
  };
}

function normalizeSamplerPlayMode(mode: SamplerPlayMode | undefined): SamplerPlayMode {
  if (mode === "hold" || mode === "loop" || mode === "oneshot") {
    return mode;
  }
  return DEFAULT_SAMPLER_PLAY_MODE;
}

function normalizeSamplerStripRoute(route: SamplerStripRoute | undefined): SamplerStripRoute {
  if (route === "after") {
    return "after";
  }
  return DEFAULT_SAMPLER_STRIP_ROUTE;
}

export function normalizeAppSettings(settings: AppSettings): AppSettings {
  const targetLufs = Number.isFinite(settings.target_lufs)
    ? settings.target_lufs
    : DEFAULT_TARGET_LUFS;
  const defaults = settings.deck_default_sampler_bank_id ?? [null, null];

  return {
    ...settings,
    master_bus: normalizeBusRoute(settings.master_bus ?? DEFAULT_MASTER_BUS),
    preview_bus: normalizeBusRoute(settings.preview_bus ?? DEFAULT_PREVIEW_BUS),
    library_table_columns: normalizeLibraryTableColumns(
      settings.library_table_columns ?? DEFAULT_LIBRARY_TABLE_COLUMNS,
    ),
    volume_normalizer_enabled:
      settings.volume_normalizer_enabled ?? DEFAULT_VOLUME_NORMALIZER_ENABLED,
    target_lufs: Math.min(MAX_TARGET_LUFS, Math.max(MIN_TARGET_LUFS, targetLufs)),
    sampler_play_mode: normalizeSamplerPlayMode(settings.sampler_play_mode),
    sampler_strip_route: normalizeSamplerStripRoute(settings.sampler_strip_route),
    deck_default_sampler_bank_id: [defaults[0] ?? null, defaults[1] ?? null],
    default_top_jog_mode: settings.default_top_jog_mode ?? DEFAULT_TOP_JOG_MODE,
    default_outer_jog_mode: settings.default_outer_jog_mode ?? DEFAULT_OUTER_JOG_MODE,
  };
}
