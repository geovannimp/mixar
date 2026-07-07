import { Pause, Play } from "lucide-react";
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
  onTogglePlayback: () => void;
}

const vinylSizeClass =
  "h-12 w-12 sm:h-16 sm:w-16 md:h-20 md:w-20";

export function DeckPanel({
  accent,
  deck,
  engineRunning,
  busy,
  onPickTrack,
  onTogglePlayback,
}: DeckPanelProps) {
  const trackTitle = deck.track ? fileName(deck.track) : "No track loaded";
  const loadDisabled = busy || !engineRunning;
  const hasTrack = Boolean(deck.track);

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
          className={`relative ${vinylSizeClass} rounded-full border-[3px] sm:border-4 ${accent.ring} bg-zinc-900/80 shadow-inner`}
        >
          {!hasTrack ? (
            <button
              type="button"
              className={`${buttonTransport} flex size-full items-center justify-center rounded-full border-white/15 bg-zinc-900/90 px-0 py-0 text-[10px] font-semibold uppercase tracking-wide hover:bg-zinc-800/90 sm:text-xs`}
              disabled={loadDisabled}
              aria-label="Load track"
              title="Load track"
              onClick={onPickTrack}
            >
              Load
            </button>
          ) : (
            <div className="group relative size-full overflow-hidden rounded-full">
              <div
                className={`absolute inset-0 bg-gradient-to-br ${accent.waveform} opacity-90`}
                aria-hidden
              />
              <div
                className={`pointer-events-none absolute left-1/2 top-1/2 flex h-[38%] w-[38%] -translate-x-1/2 -translate-y-1/2 items-center justify-center rounded-full border-2 bg-zinc-900/85 shadow-inner ${accent.ring}`}
                aria-hidden
              >
                <span className={`text-[8px] font-semibold sm:text-[10px] ${accent.text}`}>
                  {deck.playing ? "▶" : "○"}
                </span>
              </div>
              <button
                type="button"
                className={`${buttonTransport} absolute inset-0 flex items-center justify-center rounded-full bg-black/55 px-0 py-0 text-[10px] font-semibold uppercase tracking-wide opacity-0 backdrop-blur-[2px] transition-opacity group-hover:opacity-100 group-focus-within:opacity-100 hover:bg-black/65 focus:opacity-100 sm:text-xs`}
                disabled={loadDisabled}
                aria-label="Load track"
                title="Load track"
                onClick={onPickTrack}
              >
                Load
              </button>
            </div>
          )}
        </div>
      </div>

      <div className="flex justify-center">
        <button
          type="button"
          className={`${buttonTransport} inline-flex items-center justify-center px-2.5 py-1.5 sm:px-3 sm:py-2 ${accent.button}`}
          disabled={busy || !engineRunning || !deck.track}
          aria-label={deck.playing ? "Pause" : "Play"}
          title={deck.playing ? "Pause" : "Play"}
          onClick={onTogglePlayback}
        >
          {deck.playing ? (
            <Pause className="size-4 sm:size-[1.125rem]" aria-hidden />
          ) : (
            <Play className="size-4 sm:size-[1.125rem]" aria-hidden />
          )}
        </button>
      </div>
    </section>
  );
}
