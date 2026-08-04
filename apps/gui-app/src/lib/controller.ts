import { invoke } from "@tauri-apps/api/core";

export type ControllerMappingInfo = {
  id: string;
  device_id: string;
  name: string;
  midi_name_contains: string[];
  attached: boolean;
};

export type ControllerDeviceInfo = {
  port_name: string;
  direction: "input" | "output";
  matched_mapping_id: string | null;
};

export type ControllerBusEvent =
  | {
      type: "mapping_offer";
      mapping_id: string;
      device_name: string;
      port_name: string;
    }
  | { type: "mapping_attached"; mapping_id: string; port_name: string }
  | { type: "mapping_detached"; mapping_id: string };

export const CONTROLLER_EVENT = "controller://event";

export function listControllerMappings(): Promise<ControllerMappingInfo[]> {
  return invoke("controller_list_mappings");
}

export function listControllerDevices(): Promise<ControllerDeviceInfo[]> {
  return invoke("controller_list_devices");
}

export function enableControllerMapping(
  mappingId: string,
  portName?: string | null,
): Promise<void> {
  return invoke("controller_enable_mapping", {
    mappingId,
    portName: portName ?? null,
  });
}

export function disableControllerMapping(mappingId: string): Promise<void> {
  return invoke("controller_disable_mapping", { mappingId });
}

export function updateControllerMapping(mappingId: string): Promise<void> {
  return invoke("controller_update_mapping", { mappingId });
}

export function updateAllControllerMappings(): Promise<void> {
  return invoke("controller_update_all_mappings");
}
