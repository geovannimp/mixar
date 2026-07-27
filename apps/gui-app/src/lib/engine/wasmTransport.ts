import type { EngineTransport } from "@/lib/engine/transport";

/** Browser WASM host — not implemented yet (desktop uses Tauri). */
export function createWasmEngineTransport(): EngineTransport {
  throw new Error("WasmEngineTransport is not implemented yet");
}
