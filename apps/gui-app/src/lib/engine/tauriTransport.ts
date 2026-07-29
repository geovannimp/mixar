import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { EngineTransport } from "@/lib/engine/transport";
import { actionTimestampMsFromFields, cmdBodyForKind, encodeWireCmd } from "@/lib/engine/wire";

export const ENGINE_BUS_EVENT = "engine://bus";

export function createTauriEngineTransport(): EngineTransport {
  return {
    publish: (origin, kind, fields = {}) =>
      invoke("engine_publish", {
        payload: Array.from(
          encodeWireCmd(
            origin,
            kind,
            cmdBodyForKind(kind, fields),
            0,
            actionTimestampMsFromFields(fields),
          ),
        ),
      }),
    subscribe: async (handler) => {
      const unlisten = await listen<number[] | Uint8Array>(ENGINE_BUS_EVENT, (event) => {
        const payload = event.payload;
        const bytes = payload instanceof Uint8Array ? payload : Uint8Array.from(payload ?? []);
        if (bytes.length === 0) {
          return;
        }
        handler(bytes);
      });
      return () => {
        unlisten();
      };
    },
  };
}
