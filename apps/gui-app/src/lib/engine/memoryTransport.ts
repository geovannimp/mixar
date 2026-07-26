import type { EngineTransport } from "@/lib/engine/transport";

export function createMemoryEngineTransport(): EngineTransport {
  const handlers = new Set<(message: Uint8Array) => void>();

  return {
    publish(message) {
      for (const handler of handlers) {
        handler(message);
      }
      return Promise.resolve();
    },
    subscribe(handler) {
      handlers.add(handler);
      return () => {
        handlers.delete(handler);
      };
    },
  };
}
