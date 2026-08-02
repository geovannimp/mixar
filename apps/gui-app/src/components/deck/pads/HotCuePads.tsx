import { PadGridContainer } from "@/components/deck/pads/PadGridContainer";
import { DeckButton } from "@/components/ui/deck-button";
import { formatDeckTimeTenth } from "@/lib/format";
import { hotCueAccentForSlot } from "@/lib/ui";
import type { DeckHotCueMarker } from "@/types";

interface HotCuePadsProps {
  hotCues: DeckHotCueMarker[];
  disabled?: boolean;
  onTrigger: (cue: DeckHotCueMarker) => void;
  onSave: (slot: number) => void;
  onDelete: (slot: number) => void;
}

export function HotCuePads({ hotCues, disabled, onTrigger, onSave, onDelete }: HotCuePadsProps) {
  return (
    <PadGridContainer>
      {Array.from({ length: 8 }, (_, slot) => {
        const cue = hotCues.find((entry) => entry.slot === slot);
        const filled = Boolean(cue);

        return (
          <DeckButton
            key={slot}
            type="button"
            size="pad"
            accent={filled ? hotCueAccentForSlot(slot) : undefined}
            disabled={disabled}
            title={
              filled
                ? `Pad ${slot + 1} — click trigger, shift+click delete`
                : `Set hot cue on pad ${slot + 1}`
            }
            onClick={(event) => {
              if (event.shiftKey && filled) {
                onDelete(slot);
                return;
              }
              if (cue) {
                onTrigger(cue);
                return;
              }
              onSave(slot);
            }}
          >
            <span className="text-sm font-bold leading-none sm:text-base">
              {filled && cue?.label ? cue.label : slot + 1}
            </span>
            {filled ? (
              <span className="mt-0.5 text-[9px] tabular-nums opacity-75">
                {formatDeckTimeTenth(cue?.position_ms)}
              </span>
            ) : null}
          </DeckButton>
        );
      })}
    </PadGridContainer>
  );
}
