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
    <header className="flex shrink-0 items-center justify-between gap-4 border-b border-white/8 bg-zinc-900/80 px-4 py-2.5 backdrop-blur-sm">
      <div className="flex items-center gap-4">
        <h1 className="text-sm font-bold uppercase tracking-widest text-zinc-200">
          Rust DJ
        </h1>
        <span className="hidden text-xs text-zinc-500 sm:inline">
          Prototype
        </span>
      </div>

      <div className="flex items-center gap-3">
        {status?.running && (
          <span className="hidden text-xs text-zinc-500 md:inline">
            {status.backend} · {status.sample_rate} Hz
          </span>
        )}
        <StatusPill active={Boolean(status?.running)}>
          {status?.running ? "Running" : "Stopped"}
        </StatusPill>
        <button
          type="button"
          className={
            status?.running
              ? `${buttonBase} border-red-500/45 bg-red-500/15 px-3 py-1.5 text-xs hover:bg-red-500/25`
              : `${buttonBase} border-emerald-500/45 bg-emerald-500/15 px-3 py-1.5 text-xs hover:bg-emerald-500/25`
          }
          disabled={busy}
          onClick={onToggleEngine}
        >
          {status?.running ? "Stop" : "Start"}
        </button>
      </div>
    </header>
  );
}
