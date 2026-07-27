import { createMemoryEngineTransport } from "@/lib/engine/memoryTransport";
import { createTauriEngineTransport } from "@/lib/engine/tauriTransport";
import { createWasmEngineTransport } from "@/lib/engine/wasmTransport";

/** Host-agnostic engine command surface; wire encoding is transport-specific. */
export interface EngineTransport {
  play(deckId: number): Promise<void>;
  pause(deckId: number): Promise<void>;
  seek(deckId: number, positionSecs: number): Promise<void>;
  setVolume(deckId: number, volume: number): Promise<void>;
  setEq(deckId: number, low: number, mid: number, high: number): Promise<void>;
  setFilter(deckId: number, filterDb: number): Promise<void>;
  setGainTrim(deckId: number, gainDb: number): Promise<void>;
  setHeadphoneCue(deckId: number, enabled: boolean): Promise<void>;
  setCrossfader(position: number): Promise<void>;
  setCueMix(mix: number): Promise<void>;
  setMasterCue(enabled: boolean): Promise<void>;
  subscribe(handler: (message: Uint8Array) => void): () => void;
}

export type EngineBackend = "tauri" | "memory" | "wasm";

export function createEngineTransport(options?: { backend?: EngineBackend }): EngineTransport {
  const backend: EngineBackend = options?.backend ?? "tauri";
  switch (backend) {
    case "memory":
      return createMemoryEngineTransport();
    case "tauri":
      return createTauriEngineTransport();
    case "wasm":
      return createWasmEngineTransport();
    default: {
      const _exhaustive: never = backend;
      throw new Error(`Unknown engine transport backend: ${_exhaustive}`);
    }
  }
}

let sharedTransport: EngineTransport | null = null;

export function getEngineTransport(): EngineTransport {
  sharedTransport ??= createEngineTransport();
  return sharedTransport;
}
