import { configureSync, getConsoleSink, getLogger, type LogRecord } from "@logtape/logtape";
import { ENVIRONMENT } from "@/lib/tauri-app";

const isDev = import.meta.env.DEV;

function serializePropertyValue(value: unknown): unknown {
  if (value instanceof Error) {
    return { name: value.name, message: value.message, stack: value.stack };
  }
  return value;
}

export function asError(err: unknown): Error {
  return err instanceof Error ? err : new Error(String(err));
}

export function formatLogRecordForSink(record: LogRecord): string {
  const category = record.category.join(".");
  const parts: string[] = [];
  for (let i = 0; i < record.message.length; i += 1) {
    const part = record.message[i];
    parts.push(typeof part === "string" ? part : String(part));
  }
  const message = parts.join("");
  const props =
    record.properties && Object.keys(record.properties).length > 0
      ? ` ${JSON.stringify(record.properties, (_key, value) => serializePropertyValue(value))}`
      : "";
  return `[${category}] ${message}${props}`;
}

// Dynamic import keeps `@tauri-apps/plugin-log` out of the browser/wasm graph.
const tauriSink =
  ENVIRONMENT === "TAURI" ? (await import("@/lib/tauri-sink")).getTauriSink() : undefined;

/**
 * Sync LogTape setup for SPA entry (see LogTape browser/SPA guidance).
 * Runs at module evaluation so `main.tsx` can `import` this module first.
 */
if (tauriSink) {
  configureSync({
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
