import { useEffect } from "react";
import { toastManager } from "@/components/ui/toast";
import { getControllerTransport, type ControllerBusEvent } from "@/lib/controller/transport";

function showMappingOffer(payload: { mapping_id: string; device_name: string; port_name: string }) {
  const transport = getControllerTransport();
  const toastId = toastManager.add({
    id: `controller-offer:${payload.port_name}`,
    title: `${payload.device_name} connected`,
    description: "Do you want to use this controller?",
    type: "info",
    // Stay until Enable or Close — connect consent should not auto-dismiss.
    timeout: 0,
    actionProps: {
      children: "Enable",
      onClick: () => {
        toastManager.close(toastId);
        void transport.enableMapping(payload.mapping_id, payload.port_name).catch((err) => {
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
    const transport = getControllerTransport();

    const hydrate = () =>
      transport
        .pendingOffers()
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

    void transport
      .subscribe((event) => {
        if (event.type === "mapping_offer") {
          showMappingOffer(event);
        }
      })
      .then((fn) => {
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
