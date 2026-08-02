import type { EngineTransport } from "@/lib/engine/transport";

/** In-memory transport for tests; commands are no-ops until a test injects evt via subscribe. */
export function createMemoryEngineTransport(): EngineTransport {
  const handlers = new Set<(message: Uint8Array) => void>();

  return {
    publish: async () => {},
    async subscribe(handler) {
      handlers.add(handler);
      return () => {
        handlers.delete(handler);
      };
    },
  };
}
