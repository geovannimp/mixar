import { configure, getConsoleSink, getLogger, type LogRecord, type Sink } from "@logtape/logtape";
import {
  attachConsole,
  debug as tauriDebug,
  error as tauriError,
  info as tauriInfo,
  trace as tauriTrace,
  warn as tauriWarn,
} from "@tauri-apps/plugin-log";
import { isTauriApp } from "@/lib/tauri-app";

const isDev = import.meta.env.DEV;

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
      ? ` ${JSON.stringify(record.properties)}`
      : "";
  return `[${category}] ${message}${props}`;
}

function writeToTauri(record: LogRecord): void {
  const message = formatLogRecordForSink(record);
  switch (record.level) {
    case "trace":
      void tauriTrace(message);
      break;
    case "debug":
      void tauriDebug(message);
      break;
    case "info":
      void tauriInfo(message);
      break;
    case "warning":
      void tauriWarn(message);
      break;
    case "error":
    case "fatal":
      void tauriError(message);
      break;
    default: {
      const _exhaustive: never = record.level;
      void _exhaustive;
      void tauriInfo(message);
      break;
    }
  }
}

function tauriSink(): Sink {
  return (record) => {
    writeToTauri(record);
  };
}

/**
 * Configure LogTape once at app startup (before React mounts).
 *
 * Under Tauri: JS logs go through `@tauri-apps/plugin-log` so they share Stdout /
 * LogDir (and Webview) with Rust. In dev, `attachConsole()` prints that pipeline
 * in DevTools — no separate console sink, to avoid duplicates.
 *
 * Browser-only (`vite` without Tauri): console sink only.
 */
export async function configureAppLogging(): Promise<void> {
  const tauri = isTauriApp();

  if (tauri) {
    await configure({
      sinks: {
        tauri: tauriSink(),
      },
      loggers: [
        {
          category: ["gui"],
          lowestLevel: isDev ? "debug" : "info",
          sinks: ["tauri"],
        },
      ],
    });
    if (isDev) {
      await attachConsole();
    }
    return;
  }

  await configure({
    sinks: {
      console: getConsoleSink(),
    },
    loggers: [
      {
        category: ["gui"],
        lowestLevel: isDev ? "debug" : "info",
        sinks: ["console"],
      },
    ],
  });
}

export const guiLog = getLogger(["gui"]);
export const engineLog = getLogger(["gui", "engine"]);
export const libraryLog = getLogger(["gui", "library"]);
export const waveformLog = getLogger(["gui", "waveform"]);
export const controllerLog = getLogger(["gui", "controller"]);
