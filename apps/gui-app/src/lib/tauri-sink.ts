import type { LogRecord, Sink } from "@logtape/logtape";
import {
  debug as tauriDebug,
  error as tauriError,
  info as tauriInfo,
  trace as tauriTrace,
  warn as tauriWarn,
} from "@tauri-apps/plugin-log";

function serializePropertyValue(value: unknown): unknown {
  if (value instanceof Error) {
    return { name: value.name, message: value.message, stack: value.stack };
  }
  return value;
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

export function getTauriSink(): Sink {
  return (record) => {
    writeToTauri(record);
  };
}
