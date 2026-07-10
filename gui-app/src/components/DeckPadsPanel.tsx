import { DeckButton } from "@/components/ui/deck-button";
import { cn } from "@/lib/utils";
import { formatDeckTimeTenth } from "../lib/format";
import {
  BEAT_JUMP_BACK,
  BEAT_JUMP_FORWARD,
  LOOP_ROLL_BEATS,
  PAD_MODES,
  PAD_MODE_SHORT_LABELS,
} from "../lib/padModes";
import { hotCueAccentForSlot } from "../lib/ui";
import type { DeckStatus, PadMode } from "../types";

interface DeckPadsPanelProps {
  deck: DeckStatus;
  disabled?: boolean;
  onSetPadMode: (mode: PadMode) => void;
  onTriggerHotCue: (slot: number) => void;
  onSaveHotCue: (slot: number) => void;
  onDeleteHotCue: (slot: number) => void;
  onBeginLoopRoll: (beats: number) => void;
  onEndLoopRoll: () => void;
  onBeatJump: (beats: number) => void;
}

export function DeckPadsPanel({
  deck,
  disabled,
  onSetPadMode,
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
      <div
        role="tablist"
        aria-label="Pad mode"
        className="grid shrink-0 grid-cols-3 border-b border-white/8"
      >
        {PAD_MODES.map((mode) => {
          const active = deck.pad_mode === mode;
          return (
            <button
              key={mode}
              type="button"
              role="tab"
              aria-selected={active}
              disabled={controlsDisabled}
              title={PAD_MODE_SHORT_LABELS[mode]}
              className={cn(
                "px-1 py-1.5 text-[9px] font-bold uppercase tracking-[0.14em] transition-colors sm:text-[10px] sm:tracking-[0.18em]",
                active
                  ? "bg-white/10 text-zinc-100"
                  : "text-zinc-500 hover:bg-white/5 hover:text-zinc-300",
                "disabled:cursor-not-allowed disabled:text-zinc-600 disabled:hover:bg-transparent",
              )}
              onClick={() => onSetPadMode(mode)}
            >
              {PAD_MODE_SHORT_LABELS[mode]}
            </button>
          );
        })}
      </div>

      <div className="grid min-h-0 flex-1 grid-cols-4 gap-1.5 p-2 sm:gap-2 sm:p-2.5">
        {Array.from({ length: 8 }, (_, slot) => {
          switch (deck.pad_mode) {
            case "hot_cue": {
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
            case "loop_roll": {
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
            case "beat_jump": {
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
            }
            default: {
              const _exhaustive: never = deck.pad_mode;
              return _exhaustive;
            }
          }
        })}
      </div>
    </div>
  );
}
