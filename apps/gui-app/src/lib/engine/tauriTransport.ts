import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { EngineTransport } from "@/lib/engine/transport";

export const ENGINE_BUS_EVENT = "engine://bus";

export function createTauriEngineTransport(): EngineTransport {
  return {
    publish: (message) =>
      invoke("engine_publish", {
        payload: Array.from(message),
      }),
    subscribe: (handler) => {
      const unlistenPromise = listen<number[]>(ENGINE_BUS_EVENT, (event) => {
        handler(Uint8Array.from(event.payload));
      });
      return () => {
        void unlistenPromise.then((unlisten) => unlisten());
      };
    },
  };
}
