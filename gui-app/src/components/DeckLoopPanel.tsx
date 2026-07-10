import { useState } from "react";
import { DeckButton } from "@/components/ui/deck-button";
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
  onSaveLoop: (slot: number) => void;
  onRecallSavedLoop: (slot: number) => void;
  onDeleteLoop: (slot: number) => void;
  onBeatJump: (beats: number) => void;
}

export function DeckLoopPanel({
  deck,
  disabled,
  onAutoLoop,
  onLoopIn,
  onLoopOut,
  onExitLoop,
  onSaveLoop,
  onRecallSavedLoop,
  onDeleteLoop,
  onBeatJump,
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
  const loopSlot = Math.min(7, resolvedLoopBeatIndex);
  const savedLoop = deck.saved_loops.find((loop) => loop.slot === loopSlot);

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
        <DeckButton
          type="button"
          active={loopActive}
          size="cellWide"
          disabled={controlsDisabled}
          title={
            loopActive
              ? "Disable loop — shift+click to save to slot"
              : "Enable auto loop"
          }
          onClick={(event) => {
            if (event.shiftKey && loopActive) {
              onSaveLoop(loopSlot);
              return;
            }
            if (loopActive) {
              onExitLoop();
              return;
            }
            onAutoLoop(loopBeats);
          }}
        >
          Loop
        </DeckButton>

        <div className="grid grid-cols-3 gap-1">
          <DeckButton
            type="button"
            active={loopActive}
            size="cell"
            disabled={controlsDisabled || resolvedLoopBeatIndex <= 0}
            title="Halve loop length"
            onClick={() => {
              const nextIndex = Math.max(0, resolvedLoopBeatIndex - 1);
              const nextBeats = AUTO_LOOP_BEATS[nextIndex] ?? 4;
              setLoopLength(nextBeats);
            }}
          >
            ‹
          </DeckButton>
          <DeckButton
            type="button"
            active={Boolean(savedLoop)}
            size="cell"
            disabled={controlsDisabled}
            className="text-[11px] font-medium tabular-nums"
            title={
              savedLoop
                ? "Click to recall saved loop — shift+click to delete"
                : "Loop length in beats"
            }
            onClick={(event) => {
              if (!savedLoop) {
                return;
              }
              if (event.shiftKey) {
                onDeleteLoop(loopSlot);
                return;
              }
              onRecallSavedLoop(loopSlot);
            }}
          >
            {loopBeats}
          </DeckButton>
          <DeckButton
            type="button"
            active={loopActive}
            size="cell"
            disabled={
              controlsDisabled ||
              resolvedLoopBeatIndex >= AUTO_LOOP_BEATS.length - 1
            }
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
          </DeckButton>
        </div>

        <div className="grid grid-cols-2 gap-1">
          <DeckButton
            type="button"
            active={loopActive}
            size="cell"
            className="text-[9px] font-bold uppercase tracking-wide"
            disabled={controlsDisabled}
            onClick={onLoopIn}
          >
            IN
          </DeckButton>
          <DeckButton
            type="button"
            active={loopActive}
            size="cell"
            className="text-[9px] font-bold uppercase tracking-wide"
            disabled={controlsDisabled}
            onClick={onLoopOut}
          >
            OUT
          </DeckButton>
        </div>
        <div className="grid grid-cols-2 gap-1">
          <DeckButton
            type="button"
            size="cell"
            className="text-[9px] font-bold uppercase tracking-wide"
            disabled={controlsDisabled}
            title="Jump back 4 beats"
            onClick={() => onBeatJump(-4)}
          >
            -4
          </DeckButton>
          <DeckButton
            type="button"
            size="cell"
            className="text-[9px] font-bold uppercase tracking-wide"
            disabled={controlsDisabled}
            title="Jump forward 4 beats"
            onClick={() => onBeatJump(4)}
          >
            +4
          </DeckButton>
        </div>
      </div>
    </div>
  );
}
