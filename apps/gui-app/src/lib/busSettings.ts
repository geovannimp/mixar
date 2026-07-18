import type { AppSettings, BusChannelMode, BusRouteSettings } from "@/types";
import { DEFAULT_LIBRARY_TABLE_COLUMNS, normalizeLibraryTableColumns } from "./libraryTable";

export const DEFAULT_DEVICE_ID = "default";
export const DEFAULT_VOLUME_NORMALIZER_ENABLED = true;
export const DEFAULT_TARGET_LUFS = -18;
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

export function normalizeAppSettings(settings: AppSettings): AppSettings {
  const targetLufs = Number.isFinite(settings.target_lufs)
    ? settings.target_lufs
    : DEFAULT_TARGET_LUFS;

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
  };
}
