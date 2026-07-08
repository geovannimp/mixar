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

const coverClass =
  "aspect-square h-[min(5rem,40%)] min-h-15 w-auto shrink-0 overflow-hidden rounded-full border-[3px] sm:border-4 @max-h-48/controls:h-[min(3.5rem,70%)] @max-h-48/controls:min-h-15";

interface DeckCoverProps {
  accent: (typeof DECK_ACCENTS)[DeckAccent];
  hasTrack: boolean;
  playing: boolean;
  loadDisabled: boolean;
  onPickTrack: () => void;
}

function DeckCover({
  accent,
  hasTrack,
  playing,
  loadDisabled,
  onPickTrack,
}: DeckCoverProps) {
  return (
    <div
      className={`${coverClass} ${accent.ring} bg-zinc-900/80 shadow-inner`}
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
            <span
              className={`text-[8px] font-semibold sm:text-[10px] ${accent.text}`}
            >
              {playing ? "▶" : "○"}
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
  );
}

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
      className={`flex h-full min-h-0 min-w-0 flex-col gap-1.5 p-2 sm:gap-2 sm:p-3 md:gap-3 md:p-4 ${accent.bg}`}
    >
      <div className="flex items-center justify-between gap-1">
        <h2
          className={`truncate text-[10px] font-bold uppercase tracking-widest sm:text-xs ${accent.text}`}
        >
          {accent.label}
        </h2>
        <StatusPill active={deck.playing}>
          {deck.playing ? "Playing" : "Idle"}
        </StatusPill>
      </div>

      <p
        className="shrink-0 truncate rounded border border-white/8 bg-black/30 px-2 py-1 text-[11px] font-medium text-zinc-200 sm:text-xs"
        title={deck.track ?? undefined}
      >
        {trackTitle}
      </p>

      <div className="@container/controls flex min-h-0 flex-1 flex-col items-center justify-center gap-2 overflow-hidden py-1 [container-type:size] @max-h-48/controls:flex-row @max-h-48/controls:justify-center @max-h-48/controls:gap-3 sm:py-2">
        <DeckCover
          accent={accent}
          hasTrack={hasTrack}
          playing={deck.playing}
          loadDisabled={loadDisabled}
          onPickTrack={onPickTrack}
        />

        <button
          type="button"
          className={`${buttonTransport} inline-flex shrink-0 items-center justify-center px-2.5 py-1.5 sm:px-3 sm:py-2 ${accent.button}`}
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
