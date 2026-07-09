import { ChevronLeft, ChevronRight } from "lucide-react";
import { DeckButton } from "@/components/ui/deck-button";
import { formatDeckTimeTenth } from "../lib/format";
import { hotCueAccentForSlot } from "../lib/ui";
import type { DeckStatus } from "../types";

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
        <DeckButton
          type="button"
          size="icon"
          disabled
          title="Previous pad mode (coming soon)"
          aria-label="Previous pad mode"
        >
          <ChevronLeft className="size-4" aria-hidden />
        </DeckButton>
        <span className="text-center text-[10px] font-bold uppercase tracking-[0.22em] text-zinc-300">
          {PAD_MODES[padModeIndex]}
        </span>
        <DeckButton
          type="button"
          size="icon"
          disabled
          title="Next pad mode (coming soon)"
          aria-label="Next pad mode"
        >
          <ChevronRight className="size-4" aria-hidden />
        </DeckButton>
      </div>

      <div className="grid min-h-0 flex-1 grid-cols-4 gap-1.5 p-2 sm:gap-2 sm:p-2.5">
        {hotCueSlots.map((cue, slot) => {
          const filled = Boolean(cue);

          return (
            <DeckButton
              key={slot}
              type="button"
              size="pad"
              accent={filled ? hotCueAccentForSlot(slot) : undefined}
              disabled={controlsDisabled}
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
              {filled ? (
                <span className="mt-0.5 text-[9px] tabular-nums opacity-75">
                  {formatDeckTimeTenth(cue?.position_secs)}
                </span>
              ) : null}
            </DeckButton>
          );
        })}
      </div>
    </div>
  );
}
