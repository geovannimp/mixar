import {
  configure,
  configureSync,
  getConsoleSink,
  getLogger,
  type LogRecord,
} from "@logtape/logtape";
import { isTauriApp } from "@/lib/tauri-app";

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

/**
 * Sync LogTape setup for SPA entry (see LogTape browser/SPA guidance).
 * Runs at module evaluation so `main.tsx` can `import` this module first.
 */
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

/**
 * When running under Tauri, swap to the plugin sink (Stdout/LogDir/Webview) and
 * attach DevTools forwarding in dev. Lazy-imported so non-Tauri/wasm builds do
 * not pull `@tauri-apps/plugin-log` into the module graph.
 */
export async function attachTauriLogging(): Promise<void> {
  if (!isTauriApp()) {
    return;
  }

  // Dynamic import keeps the Tauri plugin out of the browser/wasm bundle graph.
  const { attachConsole, createTauriSink } = await import("@/lib/logging-tauri");

  await configure({
    reset: true,
    sinks: {
      tauri: createTauriSink(),
    },
    loggers: [
      {
        category: ["app"],
        lowestLevel: isDev ? "debug" : "info",
        sinks: ["tauri"],
      },
    ],
  });

  if (isDev) {
    await attachConsole();
  }
}

export const appLogger = getLogger(["app"]);
export const engineLogger = getLogger(["app", "engine"]);
export const libraryLogger = getLogger(["app", "library"]);
export const waveformLogger = getLogger(["app", "waveform"]);
export const controllerLogger = getLogger(["app", "controller"]);
