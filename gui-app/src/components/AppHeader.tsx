import type { EngineStatus } from "../types";
import { buttonBase } from "../lib/ui";
import { StatusPill } from "./StatusPill";

interface AppHeaderProps {
  status: EngineStatus | null;
  busy: boolean;
  onToggleEngine: () => void;
}

export function AppHeader({ status, busy, onToggleEngine }: AppHeaderProps) {
  return (
    <header className="flex flex-wrap items-start justify-between gap-4">
      <div>
        <h1 className="text-2xl font-semibold tracking-tight">Rust DJ Engine</h1>
        <p className="mt-1 text-sm text-zinc-400">
          Tauri prototype — library collections and two-deck playback
        </p>
      </div>

      <div className="flex flex-col items-end gap-2">
        <button
          type="button"
          className={
            status?.running
              ? `${buttonBase} border-red-500/45 bg-red-500/15 hover:bg-red-500/25`
              : `${buttonBase} border-emerald-500/45 bg-emerald-500/15 hover:bg-emerald-500/25`
          }
          disabled={busy}
          onClick={onToggleEngine}
        >
          {status?.running ? "Stop engine" : "Start engine"}
        </button>

        <div className="flex items-center gap-3">
          <StatusPill active={Boolean(status?.running)}>
            {status?.running ? "Running" : "Stopped"}
          </StatusPill>
          {status?.running && (
            <span className="text-xs text-zinc-400">
              {status.backend} · {status.sample_rate} Hz
            </span>
          )}
        </div>
      </div>
    </header>
  );
}
