import type { AppSettings, BusRouteSettings } from "../types";

export const DEFAULT_DEVICE_ID = "default";

export const DEFAULT_MASTER_BUS: BusRouteSettings = {
  device_id: DEFAULT_DEVICE_ID,
  left_channel: 1,
  right_channel: 2,
};

export const DEFAULT_PREVIEW_BUS: BusRouteSettings = {
  device_id: DEFAULT_DEVICE_ID,
  left_channel: 3,
  right_channel: 4,
};

export function normalizeAppSettings(settings: AppSettings): AppSettings {
  return {
    ...settings,
    master_bus: settings.master_bus ?? DEFAULT_MASTER_BUS,
    preview_bus: settings.preview_bus ?? DEFAULT_PREVIEW_BUS,
  };
}
