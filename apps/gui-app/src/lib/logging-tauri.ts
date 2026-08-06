import type { LogRecord, Sink } from "@logtape/logtape";
import {
  attachConsole,
  debug as tauriDebug,
  error as tauriError,
  info as tauriInfo,
  trace as tauriTrace,
  warn as tauriWarn,
} from "@tauri-apps/plugin-log";
import { formatLogRecordForSink } from "@/lib/logging";

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

export function createTauriSink(): Sink {
  return (record) => {
    writeToTauri(record);
  };
}

export { attachConsole };
