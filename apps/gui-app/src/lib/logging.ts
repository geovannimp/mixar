import { configure, configureSync, getConsoleSink, getLogger } from "@logtape/logtape";
import { APP_ENVIRONMENT } from "@/lib/tauri-app";

const isDev = import.meta.env.DEV;

export function asError(err: unknown): Error {
  return err instanceof Error ? err : new Error(String(err));
}

// Dynamic import keeps `@tauri-apps/plugin-log` out of the browser/wasm graph.
const tauriSink =
  APP_ENVIRONMENT === "TAURI" ? (await import("@/lib/tauri-sink")).getTauriSink() : undefined;

/**
 * LogTape SPA entry setup. Runs at module evaluation so `main.tsx` can import
 * this module first. Tauri uses `configure()` because `fromAsyncSink` needs it;
 * browser-only stays on `configureSync()`.
 */
if (tauriSink) {
  await configure({
    sinks: {
      console: getConsoleSink(),
      tauri: tauriSink,
    },
    loggers: [
      {
        category: ["app"],
        lowestLevel: isDev ? "debug" : "info",
        sinks: ["console", "tauri"],
      },
    ],
  });
} else {
  configureSync({
    sinks: {
      console: getConsoleSink(),
    },
    loggers: [
      {
        category: ["app"],
        lowestLevel: isDev ? "debug" : "info",
        sinks: ["console"],
      },
    ],
  });
}

export const appLogger = getLogger(["app"]);
export const engineLogger = getLogger(["app", "engine"]);
export const libraryLogger = getLogger(["app", "library"]);
export const waveformLogger = getLogger(["app", "waveform"]);
export const controllerLogger = getLogger(["app", "controller"]);
