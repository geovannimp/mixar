import { PadGridContainer } from "@/components/deck/pads/pad-grid-container";
import { DeckButton } from "@/components/ui/deck-button";
import { BEAT_JUMP_BACK, BEAT_JUMP_FORWARD } from "@/lib/pad-modes";

interface BeatJumpPadsProps {
  disabled?: boolean;
  onBeatJump: (beats: number) => void;
}

export function BeatJumpPads({ disabled, onBeatJump }: BeatJumpPadsProps) {
  return (
    <PadGridContainer>
      {Array.from({ length: 8 }, (_, slot) => {
        const beats = slot < 4 ? (BEAT_JUMP_FORWARD[slot] ?? 1) : (BEAT_JUMP_BACK[slot - 4] ?? -1);
        const forward = beats > 0;

        return (
          <DeckButton
            key={slot}
            type="button"
            size="pad"
            disabled={disabled}
            title={`Beat jump ${forward ? "+" : ""}${beats}`}
            onClick={() => onBeatJump(beats)}
          >
            <span className="text-sm font-bold leading-none sm:text-base">
              {forward ? `+${beats}` : beats}
            </span>
            <span className="mt-0.5 text-[9px] uppercase opacity-75">beat</span>
          </DeckButton>
        );
      })}
    </PadGridContainer>
  );
}
