import type { AppSettings, BusChannelMode, BusRouteSettings } from "../types";
import { DEFAULT_LIBRARY_TABLE_COLUMNS, normalizeLibraryTableColumns } from "./libraryTable";

export const DEFAULT_DEVICE_ID = "default";

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
  return {
    ...settings,
    master_bus: normalizeBusRoute(settings.master_bus ?? DEFAULT_MASTER_BUS),
    preview_bus: normalizeBusRoute(settings.preview_bus ?? DEFAULT_PREVIEW_BUS),
    library_table_columns: normalizeLibraryTableColumns(
      settings.library_table_columns ?? DEFAULT_LIBRARY_TABLE_COLUMNS,
    ),
  };
}
