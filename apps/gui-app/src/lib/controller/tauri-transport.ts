import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type {
  ControllerBusEvent,
  ControllerDeviceInfo,
  ControllerMappingInfo,
  ControllerTransport,
} from "@/lib/controller/transport";

export const CONTROLLER_EVENT = "controller://event";

export function createTauriControllerTransport(): ControllerTransport {
  return {
    listMappings: () => invoke<ControllerMappingInfo[]>("controller_list_mappings"),
    listDevices: () => invoke<ControllerDeviceInfo[]>("controller_list_devices"),
    pendingOffers: () => invoke<ControllerBusEvent[]>("controller_pending_offers"),
    enableMapping: (mappingId, portName) =>
      invoke("controller_enable_mapping", {
        mappingId,
        portName: portName ?? null,
      }),
    disableMapping: (mappingId) =>
      invoke("controller_disable_mapping", {
        mappingId,
      }),
    updateMapping: (mappingId) =>
      invoke("controller_update_mapping", {
        mappingId,
      }),
    updateAllMappings: () => invoke("controller_update_all_mappings"),
    subscribe: async (handler) => {
      const unlisten = await listen<ControllerBusEvent>(CONTROLLER_EVENT, (event) => {
        handler(event.payload);
      });
      return () => {
        unlisten();
      };
    },
  };
}
