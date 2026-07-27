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
      const unlistenPromise = listen<number[] | Uint8Array>(ENGINE_BUS_EVENT, (event) => {
        const payload = event.payload;
        const bytes = payload instanceof Uint8Array ? payload : Uint8Array.from(payload ?? []);
        if (bytes.length === 0) {
          return;
        }
        handler(bytes);
      });
      return () => {
        void unlistenPromise.then((unlisten) => unlisten());
      };
    },
  };
}
