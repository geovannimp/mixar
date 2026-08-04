import { listen } from "@tauri-apps/api/event";
import { useEffect } from "react";
import { toastManager } from "@/components/ui/toast";
import {
  CONTROLLER_EVENT,
  enableControllerMapping,
  type ControllerBusEvent,
} from "@/lib/controller";

/** Listens for MIDI mapping offers and prompts to enable. */
export function ControllerOfferBridge() {
  useEffect(() => {
    let cancelled = false;
    let unlisten: (() => void) | undefined;

    void listen<ControllerBusEvent>(CONTROLLER_EVENT, (event) => {
      const payload = event.payload;
      if (payload.type !== "mapping_offer") {
        return;
      }
      toastManager.add({
        title: `Controller: ${payload.device_name}`,
        description: `Enable mapping for ${payload.port_name}?`,
        type: "info",
        actionProps: {
          children: "Enable",
          onClick: () => {
            void enableControllerMapping(payload.mapping_id, payload.port_name).catch((err) => {
              toastManager.add({
                title: err instanceof Error ? err.message : String(err),
                type: "error",
              });
            });
          },
        },
      });
    }).then((fn) => {
      if (cancelled) {
        fn();
        return;
      }
      unlisten = fn;
    });

    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, []);

  return null;
}
