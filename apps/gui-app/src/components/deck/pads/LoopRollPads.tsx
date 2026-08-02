import { PadGridContainer } from "@/components/deck/pads/PadGridContainer";
import { DeckButton } from "@/components/ui/deck-button";
import { LOOP_ROLL_BEATS } from "@/lib/padModes";

interface LoopRollPadsProps {
  disabled?: boolean;
  onBegin: (beats: number) => void;
  onEnd: () => void;
}

export function LoopRollPads({ disabled, onBegin, onEnd }: LoopRollPadsProps) {
  return (
    <PadGridContainer>
      {Array.from({ length: 8 }, (_, slot) => {
        const beats = LOOP_ROLL_BEATS[slot] ?? 4;
        return (
          <DeckButton
            key={slot}
            type="button"
            size="pad"
            disabled={disabled}
            title={`Loop roll ${beats} beat${beats === 1 ? "" : "s"} — hold`}
            onPointerDown={() => onBegin(beats)}
            onPointerUp={() => onEnd()}
            onPointerLeave={() => onEnd()}
          >
            <span className="text-sm font-bold leading-none sm:text-base">{beats}</span>
            <span className="mt-0.5 text-[9px] uppercase opacity-75">roll</span>
          </DeckButton>
        );
      })}
    </PadGridContainer>
  );
}
