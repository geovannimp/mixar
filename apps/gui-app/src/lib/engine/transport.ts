import { createMemoryEngineTransport } from "@/lib/engine/memoryTransport";
import { createTauriEngineTransport } from "@/lib/engine/tauriTransport";

export interface EngineTransport {
  publish(message: Uint8Array): Promise<void>;
  subscribe(handler: (message: Uint8Array) => void): () => void;
}

export function createEngineTransport(options?: { backend?: "tauri" | "memory" }): EngineTransport {
  if (options?.backend === "memory") {
    return createMemoryEngineTransport();
  }
  return createTauriEngineTransport();
}
