import { buttonBase } from "../lib/ui";
import { fileName } from "../lib/format";
import type { DeckStatus } from "../types";
import { StatusPill } from "./StatusPill";

interface DeckPanelProps {
  label: string;
  deck: DeckStatus;
  engineRunning: boolean;
  busy: boolean;
  onPickTrack: () => void;
  onLoadSample: () => void;
  onPlay: () => void;
  onPause: () => void;
}

export function DeckPanel({
  label,
  deck,
  engineRunning,
  busy,
  onPickTrack,
  onLoadSample,
  onPlay,
  onPause,
}: DeckPanelProps) {
  return (
    <section className="flex min-h-64 flex-col gap-4 rounded-2xl border border-white/8 bg-white/3 p-5">
      <div className="flex items-center justify-between">
        <h2 className="text-lg font-semibold">{label}</h2>
        <StatusPill active={deck.playing}>
          {deck.playing ? "Playing" : "Idle"}
        </StatusPill>
      </div>

      <p
        className="truncate rounded-lg border border-dashed border-white/12 bg-black/35 px-4 py-3 text-sm text-slate-300"
        title={deck.track ?? undefined}
      >
        {fileName(deck.track)}
      </p>

      <div className="flex flex-wrap gap-2">
        <button
          type="button"
          className={`${buttonBase} border-white/12 bg-white/6 hover:border-white/20 hover:bg-white/10`}
          disabled={busy || !engineRunning}
          onClick={onPickTrack}
        >
          Load file
        </button>
        <button
          type="button"
          className={`${buttonBase} border-sky-500/35 bg-sky-500/12 hover:bg-sky-500/20`}
          disabled={busy || !engineRunning}
          onClick={onLoadSample}
        >
          Load sample
        </button>
      </div>

      <div className="flex flex-wrap gap-2">
        <button
          type="button"
          className={`${buttonBase} border-violet-500/45 bg-violet-500/15 hover:bg-violet-500/25`}
          disabled={busy || !engineRunning || !deck.track}
          onClick={onPlay}
        >
          Play
        </button>
        <button
          type="button"
          className={`${buttonBase} border-white/12 bg-white/6 hover:border-white/20 hover:bg-white/10`}
          disabled={busy || !engineRunning || !deck.playing}
          onClick={onPause}
        >
          Pause
        </button>
      </div>
    </section>
  );
}
