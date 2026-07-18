import { useEffect } from "react";
import type { PadMode } from "../types";

interface UseDeckHotkeysOptions {
  focusedDeckId: number;
  padMode: PadMode;
  onTriggerHotCue: (deckId: number, slot: number) => void;
  onBeatJump: (deckId: number, beats: number) => void;
  onBeginLoopRoll: (deckId: number, beats: number) => void;
  onEndLoopRoll: (deckId: number) => void;
}

function isEditableTarget(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) {
    return false;
  }
  const tag = target.tagName;
  return target.isContentEditable || tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT";
}

const LOOP_ROLL_BEATS = [1, 2, 4, 8, 16, 32, 64, 128] as const;
const BEAT_JUMP_FORWARD = [1, 2, 4, 8, 16, 32, 64, 128] as const;
const BEAT_JUMP_BACK = [-1, -2, -4, -8, -16, -32, -64, -128] as const;

export function useDeckHotkeys({
  focusedDeckId,
  padMode,
  onTriggerHotCue,
  onBeatJump,
  onBeginLoopRoll,
  onEndLoopRoll,
}: UseDeckHotkeysOptions): void {
  useEffect(() => {
    const heldLoopRollSlots = new Set<number>();

    const onKeyDown = (event: KeyboardEvent) => {
      if (event.defaultPrevented || isEditableTarget(event.target)) {
        return;
      }
      if (event.ctrlKey || event.metaKey || event.altKey) {
        return;
      }

      const slot = Number.parseInt(event.key, 10);
      if (!Number.isFinite(slot) || slot < 1 || slot > 8) {
        return;
      }

      event.preventDefault();
      const slotIndex = slot - 1;

      switch (padMode) {
        case "hot_cue":
          onTriggerHotCue(focusedDeckId, slotIndex);
          break;
        case "beat_jump": {
          const beats =
            slotIndex < 4
              ? (BEAT_JUMP_FORWARD[slotIndex] ?? 1)
              : (BEAT_JUMP_BACK[slotIndex - 4] ?? -1);
          onBeatJump(focusedDeckId, beats);
          break;
        }
        case "loop_roll": {
          if (heldLoopRollSlots.has(slotIndex)) {
            return;
          }
          heldLoopRollSlots.add(slotIndex);
          onBeginLoopRoll(focusedDeckId, LOOP_ROLL_BEATS[slotIndex] ?? 4);
          break;
        }
        default: {
          const _exhaustive: never = padMode;
          return _exhaustive;
        }
      }
    };

    const onKeyUp = (event: KeyboardEvent) => {
      if (padMode !== "loop_roll") {
        return;
      }
      const slot = Number.parseInt(event.key, 10);
      if (!Number.isFinite(slot) || slot < 1 || slot > 8) {
        return;
      }
      const slotIndex = slot - 1;
      if (!heldLoopRollSlots.delete(slotIndex)) {
        return;
      }
      onEndLoopRoll(focusedDeckId);
    };

    window.addEventListener("keydown", onKeyDown);
    window.addEventListener("keyup", onKeyUp);
    return () => {
      window.removeEventListener("keydown", onKeyDown);
      window.removeEventListener("keyup", onKeyUp);
    };
  }, [focusedDeckId, onBeatJump, onBeginLoopRoll, onEndLoopRoll, onTriggerHotCue, padMode]);
}
