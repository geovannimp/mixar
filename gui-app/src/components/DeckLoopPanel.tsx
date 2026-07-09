import { useState } from "react";
import { cn } from "@/lib/utils";
import type { DeckStatus } from "../types";

const AUTO_LOOP_BEATS = [1, 2, 4, 8, 16, 32] as const;

interface DeckLoopPanelProps {
  deck: DeckStatus;
  disabled?: boolean;
  onAutoLoop: (beats: number) => void;
  onLoopIn: () => void;
  onLoopOut: () => void;
  onExitLoop: () => void;
}

export function DeckLoopPanel({
  deck,
  disabled,
  onAutoLoop,
  onLoopIn,
  onLoopOut,
  onExitLoop,
}: DeckLoopPanelProps) {
  const hasTrack = Boolean(deck.track);
  const controlsDisabled = disabled || !hasTrack;
  const loopActive = Boolean(deck.active_loop?.active);
  const [loopBeats, setLoopBeats] = useState(4);
  const loopBeatIndex = AUTO_LOOP_BEATS.indexOf(
    loopBeats as (typeof AUTO_LOOP_BEATS)[number],
  );
  const resolvedLoopBeatIndex =
    loopBeatIndex >= 0 ? loopBeatIndex : AUTO_LOOP_BEATS.indexOf(4);

  const controlClass = loopActive
    ? "border-emerald-500/35 bg-emerald-950/50 text-emerald-100 hover:bg-emerald-900/55"
    : "border-white/12 bg-zinc-900/90 text-zinc-300 hover:bg-zinc-800";

  const cellClass = cn(
    "flex h-8 min-w-0 items-center justify-center rounded border px-0 py-0 transition disabled:cursor-not-allowed disabled:opacity-45 sm:h-9",
    controlClass,
  );

  const setLoopLength = (beats: number) => {
    setLoopBeats(beats);
    onAutoLoop(beats);
  };

  return (
    <div
      className={cn(
        "flex w-22 shrink-0 flex-col overflow-hidden rounded-md border shadow-inner sm:w-24",
        loopActive
          ? "border-emerald-500/45 bg-emerald-950/35"
          : "border-white/10 bg-zinc-950/80",
      )}
    >
      <div className="flex shrink-0 flex-col gap-1 p-1.5">
        <button
          type="button"
          className={cn(
            "flex h-8 w-full items-center justify-center rounded border px-1 text-[9px] font-bold uppercase tracking-[0.2em] transition disabled:cursor-not-allowed disabled:opacity-45 sm:h-9",
            loopActive
              ? "border-emerald-400/55 bg-emerald-500/25 text-emerald-100 hover:bg-emerald-500/35"
              : controlClass,
          )}
          disabled={controlsDisabled}
          title={loopActive ? "Disable loop" : "Enable auto loop"}
          onClick={() => {
            if (loopActive) {
              onExitLoop();
              return;
            }
            onAutoLoop(loopBeats);
          }}
        >
          Loop
        </button>

        <div className="grid grid-cols-3 gap-1">
          <button
            type="button"
            disabled={controlsDisabled || resolvedLoopBeatIndex <= 0}
            className={cn(cellClass, "text-xs font-semibold")}
            title="Halve loop length"
            onClick={() => {
              const nextIndex = Math.max(0, resolvedLoopBeatIndex - 1);
              const nextBeats = AUTO_LOOP_BEATS[nextIndex] ?? 4;
              setLoopLength(nextBeats);
            }}
          >
            ‹
          </button>
          <span
            className="flex h-8 min-w-0 items-center justify-center text-[11px] font-medium tabular-nums text-zinc-500 sm:h-9"
            title="Loop length in beats (reference)"
          >
            {loopBeats}
          </span>
          <button
            type="button"
            disabled={
              controlsDisabled ||
              resolvedLoopBeatIndex >= AUTO_LOOP_BEATS.length - 1
            }
            className={cn(cellClass, "text-xs font-semibold")}
            title="Double loop length"
            onClick={() => {
              const nextIndex = Math.min(
                AUTO_LOOP_BEATS.length - 1,
                resolvedLoopBeatIndex + 1,
              );
              const nextBeats = AUTO_LOOP_BEATS[nextIndex] ?? 4;
              setLoopLength(nextBeats);
            }}
          >
            ›
          </button>
        </div>

        <div className="grid grid-cols-2 gap-1">
          <button
            type="button"
            disabled={controlsDisabled}
            className={cn(cellClass, "text-[9px] font-bold uppercase tracking-wide")}
            onClick={onLoopIn}
          >
            IN
          </button>
          <button
            type="button"
            disabled={controlsDisabled}
            className={cn(cellClass, "text-[9px] font-bold uppercase tracking-wide")}
            onClick={onLoopOut}
          >
            OUT
          </button>
        </div>
      </div>
    </div>
  );
}
