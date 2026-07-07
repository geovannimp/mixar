import { NavLink } from "react-router-dom";
import type { EngineStatus } from "../types";
import { buttonBase } from "../lib/ui";
import { StatusPill } from "./StatusPill";

interface AppHeaderProps {
  status: EngineStatus | null;
  busy: boolean;
  onToggleEngine: () => void;
}

function navClass({ isActive }: { isActive: boolean }): string {
  return isActive
    ? "rounded border border-white/20 bg-white/10 px-2.5 py-1 text-xs font-semibold uppercase tracking-wide text-zinc-100"
    : "rounded border border-transparent px-2.5 py-1 text-xs font-semibold uppercase tracking-wide text-zinc-500 hover:text-zinc-300";
}

export function AppHeader({ status, busy, onToggleEngine }: AppHeaderProps) {
  return (
    <header className="flex shrink-0 items-center justify-between gap-4 border-b border-white/8 bg-zinc-900/80 px-4 py-2.5 backdrop-blur-sm">
      <div className="flex min-w-0 items-center gap-3 sm:gap-4">
        <h1 className="shrink-0 text-sm font-bold uppercase tracking-widest text-zinc-200">
          Rust DJ
        </h1>
        <nav className="flex items-center gap-1">
          <NavLink to="/" end className={navClass}>
            Decks
          </NavLink>
          <NavLink to="/settings" className={navClass}>
            Settings
          </NavLink>
        </nav>
      </div>

      <div className="flex shrink-0 items-center gap-3">
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
