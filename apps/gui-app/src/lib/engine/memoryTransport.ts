import type { EngineTransport } from "@/lib/engine/transport";

const noop = async (): Promise<void> => {};

/** In-memory transport for tests; commands are no-ops until a test injects evt via subscribe. */
export function createMemoryEngineTransport(): EngineTransport {
  const handlers = new Set<(message: Uint8Array) => void>();

  return {
    play: noop,
    pause: noop,
    seek: noop,
    setVolume: noop,
    setEq: noop,
    setFilter: noop,
    setGainTrim: noop,
    setHeadphoneCue: noop,
    setCrossfader: noop,
    setCueMix: noop,
    setMasterCue: noop,
    subscribe(handler) {
      handlers.add(handler);
      return () => {
        handlers.delete(handler);
      };
    },
  };
}
