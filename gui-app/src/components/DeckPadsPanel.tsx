import { ChevronLeft, ChevronRight } from "lucide-react";
import { cn } from "@/lib/utils";
import { buttonIcon } from "../lib/ui";
import type { DeckStatus } from "../types";

const HOT_CUE_COLORS = [
  "border-red-500/55 bg-red-500/20 text-red-100",
  "border-orange-500/55 bg-orange-500/20 text-orange-100",
  "border-yellow-500/55 bg-yellow-500/20 text-yellow-100",
  "border-green-500/55 bg-green-500/20 text-green-100",
  "border-cyan-500/55 bg-cyan-500/20 text-cyan-100",
  "border-blue-500/55 bg-blue-500/20 text-blue-100",
  "border-violet-500/55 bg-violet-500/20 text-violet-100",
  "border-pink-500/55 bg-pink-500/20 text-pink-100",
] as const;

const PAD_MODES = ["Hot Cue"] as const;

interface DeckPadsPanelProps {
  deck: DeckStatus;
  disabled?: boolean;
  onTriggerHotCue: (slot: number) => void;
  onSaveHotCue: (slot: number) => void;
  onDeleteHotCue: (slot: number) => void;
}

export function DeckPadsPanel({
  deck,
  disabled,
  onTriggerHotCue,
  onSaveHotCue,
  onDeleteHotCue,
}: DeckPadsPanelProps) {
  const hasTrack = Boolean(deck.track);
  const controlsDisabled = disabled || !hasTrack;
  const padModeIndex = 0;

  const hotCueSlots = Array.from({ length: 8 }, (_, slot) => {
    return deck.hot_cues.find((cue) => cue.slot === slot);
  });

  return (
    <div className="flex min-w-0 flex-1 flex-col overflow-hidden rounded-md border border-white/10 bg-zinc-950/80 shadow-inner">
      <div className="grid shrink-0 grid-cols-[auto_1fr_auto] items-center gap-1 border-b border-white/8 px-2 py-1.5">
        <button
          type="button"
          disabled
          className={cn(
            buttonIcon,
            "h-7 w-7 border-white/10 bg-zinc-900/80 text-zinc-600",
          )}
          title="Previous pad mode (coming soon)"
          aria-label="Previous pad mode"
        >
          <ChevronLeft className="size-4" aria-hidden />
        </button>
        <span className="text-center text-[10px] font-bold uppercase tracking-[0.22em] text-zinc-300">
          {PAD_MODES[padModeIndex]}
        </span>
        <button
          type="button"
          disabled
          className={cn(
            buttonIcon,
            "h-7 w-7 border-white/10 bg-zinc-900/80 text-zinc-600",
          )}
          title="Next pad mode (coming soon)"
          aria-label="Next pad mode"
        >
          <ChevronRight className="size-4" aria-hidden />
        </button>
      </div>

      <div className="grid min-h-0 flex-1 grid-cols-4 gap-1.5 p-2 sm:gap-2 sm:p-2.5">
        {hotCueSlots.map((cue, slot) => {
          const filled = Boolean(cue);
          const colorClass =
            HOT_CUE_COLORS[slot % HOT_CUE_COLORS.length] ?? HOT_CUE_COLORS[0];

          return (
            <button
              key={slot}
              type="button"
              disabled={controlsDisabled}
              className={cn(
                "flex min-h-11 flex-col items-center justify-center rounded-md border px-1 py-1.5 text-center transition sm:min-h-12",
                "disabled:cursor-not-allowed disabled:opacity-45",
                filled
                  ? colorClass
                  : "border-white/12 bg-zinc-900/90 text-zinc-500 hover:bg-zinc-800/90",
              )}
              title={
                filled
                  ? `Pad ${slot + 1} — click trigger, shift+click delete`
                  : `Set hot cue on pad ${slot + 1}`
              }
              onClick={(event) => {
                if (event.shiftKey && filled) {
                  onDeleteHotCue(slot);
                  return;
                }
                if (filled) {
                  onTriggerHotCue(slot);
                  return;
                }
                onSaveHotCue(slot);
              }}
            >
              <span className="text-sm font-bold leading-none sm:text-base">
                {filled && cue?.label ? cue.label : slot + 1}
              </span>
              {filled && cue?.label ? (
                <span className="mt-0.5 text-[9px] font-medium uppercase tracking-wide opacity-75">
                  {slot + 1}
                </span>
              ) : null}
            </button>
          );
        })}
      </div>
    </div>
  );
}
