import { createMemoryEngineTransport } from "@/lib/engine/memoryTransport";
import { createTauriEngineTransport } from "@/lib/engine/tauriTransport";
import { createWasmEngineTransport } from "@/lib/engine/wasmTransport";

export interface EngineTransport {
  publish(message: Uint8Array): Promise<void>;
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
