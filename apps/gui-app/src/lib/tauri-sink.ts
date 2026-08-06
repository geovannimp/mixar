import { fromAsyncSink, type LogRecord, type Sink } from "@logtape/logtape";
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

async function writeToTauri(record: LogRecord): Promise<void> {
  const message = formatLogRecordForSink(record);
  switch (record.level) {
    case "trace":
      await tauriTrace(message);
      break;
    case "debug":
      await tauriDebug(message);
      break;
    case "info":
      await tauriInfo(message);
      break;
    case "warning":
      await tauriWarn(message);
      break;
    case "error":
    case "fatal":
      await tauriError(message);
      break;
    default: {
      const _exhaustive: never = record.level;
      void _exhaustive;
      await tauriInfo(message);
      break;
    }
  }
}

export function getTauriSink(): Sink {
  return fromAsyncSink(writeToTauri);
}
