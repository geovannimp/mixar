import { listen } from "@tauri-apps/api/event";
import { useEffect } from "react";
import { toastManager } from "@/components/ui/toast";
import {
  CONTROLLER_EVENT,
  enableControllerMapping,
  listControllerPendingOffers,
  type ControllerBusEvent,
} from "@/lib/controller";

function showMappingOffer(payload: { mapping_id: string; device_name: string; port_name: string }) {
  const toastId = toastManager.add({
    id: `controller-offer:${payload.port_name}`,
    title: `Controller: ${payload.device_name}`,
    description: `Enable mapping for ${payload.port_name}?`,
    type: "info",
    // Stay until Enable or Close — connect consent should not auto-dismiss.
    timeout: 0,
    actionProps: {
      children: "Enable",
      onClick: () => {
        toastManager.close(toastId);
        void enableControllerMapping(payload.mapping_id, payload.port_name).catch((err) => {
          toastManager.add({
            title: err instanceof Error ? err.message : String(err),
            type: "error",
          });
        });
      },
    },
  });
}

function applyOffers(events: ControllerBusEvent[]) {
  for (const event of events) {
    if (event.type === "mapping_offer") {
      showMappingOffer(event);
    }
  }
}

/** Listens for MIDI mapping offers and prompts to enable. */
export function ControllerOfferBridge() {
  useEffect(() => {
    let cancelled = false;
    let unlisten: (() => void) | undefined;
    let retryTimer: ReturnType<typeof setInterval> | undefined;

    const hydrate = () =>
      listControllerPendingOffers()
        .then((events) => {
          if (cancelled || events.length === 0) {
            return events.length;
          }
          applyOffers(events);
          return events.length;
        })
        .catch((err) => {
          console.warn("controller pending offers:", err);
          return 0;
        });

    // First MIDI scan can take several seconds on cold ALSA — retry until cached.
    void hydrate().then((count) => {
      if (cancelled || count > 0) {
        return;
      }
      let attempts = 0;
      retryTimer = setInterval(() => {
        attempts += 1;
        void hydrate().then((n) => {
          if (n > 0 || attempts >= 40) {
            if (retryTimer) {
              clearInterval(retryTimer);
              retryTimer = undefined;
            }
          }
        });
      }, 250);
    });

    void listen<ControllerBusEvent>(CONTROLLER_EVENT, (event) => {
      if (event.payload.type === "mapping_offer") {
        showMappingOffer(event.payload);
      }
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
      if (retryTimer) {
        clearInterval(retryTimer);
      }
    };
  }, []);

  return null;
}
