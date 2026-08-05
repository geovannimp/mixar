//! ControllerTransport — host-agnostic MIDI mapping surface (mirrors engine/library).

import { createTauriControllerTransport } from "@/lib/controller/tauri-transport";

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

export interface ControllerTransport {
  listMappings(): Promise<ControllerMappingInfo[]>;
  listDevices(): Promise<ControllerDeviceInfo[]>;
  pendingOffers(): Promise<ControllerBusEvent[]>;
  enableMapping(mappingId: string, portName?: string | null): Promise<void>;
  disableMapping(mappingId: string): Promise<void>;
  updateMapping(mappingId: string): Promise<void>;
  updateAllMappings(): Promise<void>;
  /** Resolves after the host listener is registered. */
  subscribe(handler: (event: ControllerBusEvent) => void): Promise<() => void>;
}

export type ControllerBackend = "tauri";

export function createControllerTransport(options?: {
  backend?: ControllerBackend;
}): ControllerTransport {
  const backend: ControllerBackend = options?.backend ?? "tauri";
  switch (backend) {
    case "tauri":
      return createTauriControllerTransport();
    default: {
      const _exhaustive: never = backend;
      throw new Error(`Unknown controller transport backend: ${String(_exhaustive)}`);
    }
  }
}

let sharedTransport: ControllerTransport | null = null;

export function getControllerTransport(): ControllerTransport {
  sharedTransport ??= createControllerTransport();
  return sharedTransport;
}

/** Test helper: swap the shared transport (pass `null` to clear). */
export function setControllerTransportForTests(transport: ControllerTransport | null): void {
  sharedTransport = transport;
}
