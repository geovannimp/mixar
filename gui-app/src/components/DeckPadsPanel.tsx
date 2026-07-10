import { ChevronLeft, ChevronRight } from "lucide-react";
import { DeckButton } from "@/components/ui/deck-button";
import { formatDeckTimeTenth } from "../lib/format";
import {
  BEAT_JUMP_BACK,
  BEAT_JUMP_FORWARD,
  LOOP_ROLL_BEATS,
  PAD_MODE_LABELS,
} from "../lib/padModes";
import { hotCueAccentForSlot } from "../lib/ui";
import type { DeckStatus, PadMode } from "../types";

interface DeckPadsPanelProps {
  deck: DeckStatus;
  disabled?: boolean;
  onCyclePadMode: (direction: number) => void;
  onTriggerHotCue: (slot: number) => void;
  onSaveHotCue: (slot: number) => void;
  onDeleteHotCue: (slot: number) => void;
  onBeginLoopRoll: (beats: number) => void;
  onEndLoopRoll: () => void;
  onBeatJump: (beats: number) => void;
}

function padModeLabel(mode: PadMode): string {
  return PAD_MODE_LABELS[mode];
}

export function DeckPadsPanel({
  deck,
  disabled,
  onCyclePadMode,
  onTriggerHotCue,
  onSaveHotCue,
  onDeleteHotCue,
  onBeginLoopRoll,
  onEndLoopRoll,
  onBeatJump,
}: DeckPadsPanelProps) {
  const hasTrack = Boolean(deck.track);
  const controlsDisabled = disabled || !hasTrack;

  const hotCueSlots = Array.from({ length: 8 }, (_, slot) => {
    return deck.hot_cues.find((cue) => cue.slot === slot);
  });

  return (
    <div className="flex min-w-0 flex-1 flex-col overflow-hidden rounded-md border border-white/10 bg-zinc-950/80 shadow-inner">
      <div className="grid shrink-0 grid-cols-[auto_1fr_auto] items-center gap-1 border-b border-white/8 px-2 py-1.5">
        <DeckButton
          type="button"
          size="icon"
          disabled={controlsDisabled}
          title="Previous pad mode"
          aria-label="Previous pad mode"
          onClick={() => onCyclePadMode(-1)}
        >
          <ChevronLeft className="size-4" aria-hidden />
        </DeckButton>
        <span className="text-center text-[10px] font-bold uppercase tracking-[0.22em] text-zinc-300">
          {padModeLabel(deck.pad_mode)}
        </span>
        <DeckButton
          type="button"
          size="icon"
          disabled={controlsDisabled}
          title="Next pad mode"
          aria-label="Next pad mode"
          onClick={() => onCyclePadMode(1)}
        >
          <ChevronRight className="size-4" aria-hidden />
        </DeckButton>
      </div>

      <div className="grid min-h-0 flex-1 grid-cols-4 gap-1.5 p-2 sm:gap-2 sm:p-2.5">
        {Array.from({ length: 8 }, (_, slot) => {
          if (deck.pad_mode === "hot_cue") {
            const cue = hotCueSlots[slot];
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
          }

          if (deck.pad_mode === "loop_roll") {
            const beats = LOOP_ROLL_BEATS[slot] ?? 4;
            return (
              <DeckButton
                key={slot}
                type="button"
                size="pad"
                disabled={controlsDisabled}
                title={`Loop roll ${beats} beat${beats === 1 ? "" : "s"} — hold`}
                onPointerDown={() => onBeginLoopRoll(beats)}
                onPointerUp={() => onEndLoopRoll()}
                onPointerLeave={() => onEndLoopRoll()}
              >
                <span className="text-sm font-bold leading-none sm:text-base">
                  {beats}
                </span>
                <span className="mt-0.5 text-[9px] uppercase opacity-75">
                  roll
                </span>
              </DeckButton>
            );
          }

          const beats =
            slot < 4
              ? (BEAT_JUMP_FORWARD[slot] ?? 1)
              : (BEAT_JUMP_BACK[slot - 4] ?? -1);
          const forward = beats > 0;

          return (
            <DeckButton
              key={slot}
              type="button"
              size="pad"
              disabled={controlsDisabled}
              title={`Beat jump ${forward ? "+" : ""}${beats}`}
              onClick={() => onBeatJump(beats)}
            >
              <span className="text-sm font-bold leading-none sm:text-base">
                {forward ? `+${beats}` : beats}
              </span>
              <span className="mt-0.5 text-[9px] uppercase opacity-75">
                beat
              </span>
            </DeckButton>
          );
        })}
      </div>
    </div>
  );
}
