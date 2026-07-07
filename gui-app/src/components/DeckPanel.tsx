import { buttonTransport, type DeckAccent, DECK_ACCENTS } from "../lib/ui";
import { fileName } from "../lib/format";
import type { DeckStatus } from "../types";
import { StatusPill } from "./StatusPill";

interface DeckPanelProps {
  accent: (typeof DECK_ACCENTS)[DeckAccent];
  deck: DeckStatus;
  engineRunning: boolean;
  busy: boolean;
  onPickTrack: () => void;
  onLoadSample: () => void;
  onPlay: () => void;
  onPause: () => void;
}

export function DeckPanel({
  accent,
  deck,
  engineRunning,
  busy,
  onPickTrack,
  onLoadSample,
  onPlay,
  onPause,
}: DeckPanelProps) {
  const trackTitle = deck.track ? fileName(deck.track) : "No track loaded";

  return (
    <section
      className={`flex min-w-0 flex-col gap-1.5 p-2 sm:gap-2 sm:p-3 md:gap-3 md:p-4 ${accent.bg}`}
    >
      <div className="flex items-center justify-between gap-1">
        <h2 className={`truncate text-[10px] font-bold uppercase tracking-widest sm:text-xs ${accent.text}`}>
          {accent.label}
        </h2>
        <StatusPill active={deck.playing}>
          {deck.playing ? "Playing" : "Idle"}
        </StatusPill>
      </div>

      <div
        className={`relative flex h-10 shrink-0 items-end overflow-hidden rounded border sm:h-12 md:h-14 ${accent.border} bg-black/40`}
        title={deck.track ?? undefined}
      >
        <div
          className={`absolute inset-0 bg-gradient-to-t ${accent.waveform} opacity-60`}
        />
        <div className="absolute inset-y-0 left-1/2 w-px bg-white/20" />
        <p className="relative z-10 truncate px-2 py-1 text-[11px] font-medium text-zinc-100 sm:text-xs md:text-sm">
          {trackTitle}
        </p>
      </div>

      <div className="flex items-center justify-center py-1 sm:py-2">
        <div
          className={`flex h-12 w-12 items-center justify-center rounded-full border-[3px] sm:h-16 sm:w-16 sm:border-4 md:h-20 md:w-20 ${accent.ring} bg-zinc-900/80 shadow-inner`}
        >
          <span className={`text-[10px] font-semibold uppercase sm:text-xs ${accent.text}`}>
            {deck.playing ? "▶" : "○"}
          </span>
        </div>
      </div>

      <div className="grid grid-cols-2 gap-1 sm:flex sm:flex-wrap sm:justify-center sm:gap-1.5">
        <button
          type="button"
          className={`${buttonTransport} px-2 py-1 text-[10px] sm:px-3 sm:py-1.5 sm:text-xs border-white/15 bg-white/5 hover:bg-white/10`}
          disabled={busy || !engineRunning}
          onClick={onPickTrack}
        >
          Load
        </button>
        <button
          type="button"
          className={`${buttonTransport} px-2 py-1 text-[10px] sm:px-3 sm:py-1.5 sm:text-xs border-white/15 bg-white/5 hover:bg-white/10`}
          disabled={busy || !engineRunning}
          onClick={onLoadSample}
        >
          Sample
        </button>
        <button
          type="button"
          className={`${buttonTransport} px-2 py-1 text-[10px] sm:px-3 sm:py-1.5 sm:text-xs ${accent.button}`}
          disabled={busy || !engineRunning || !deck.track}
          onClick={onPlay}
        >
          Play
        </button>
        <button
          type="button"
          className={`${buttonTransport} px-2 py-1 text-[10px] sm:px-3 sm:py-1.5 sm:text-xs border-white/15 bg-white/5 hover:bg-white/10`}
          disabled={busy || !engineRunning || !deck.playing}
          onClick={onPause}
        >
          Pause
        </button>
      </div>
    </section>
  );
}
