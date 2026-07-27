import { createMemoryEngineTransport } from "@/lib/engine/memoryTransport";
import { createTauriEngineTransport } from "@/lib/engine/tauriTransport";
import { createWasmEngineTransport } from "@/lib/engine/wasmTransport";
import type { CmdKind, Origin } from "@/lib/engine/wire";

/** Host-agnostic engine command surface; wire encoding is transport-specific. */
export interface EngineTransport {
  /** `fields` are CmdBody payload fields only — body `type` is derived from `kind`. */
  publish(origin: Origin, kind: CmdKind, fields?: Record<string, unknown>): Promise<void>;
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
