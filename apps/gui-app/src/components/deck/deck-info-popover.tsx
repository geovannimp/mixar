import type { ReactElement } from "react";
import { Info } from "lucide-react";
import {
  Popover,
  PopoverDescription,
  PopoverPopup,
  PopoverTitle,
  PopoverTrigger,
} from "@/components/ui/popover";
import { normToStripDb } from "@/lib/eq";
import { buttonIcon } from "@/lib/ui";
import type { DeckStatus } from "@/types";

interface DeckInfoPopoverProps {
  deck: DeckStatus;
  disabled?: boolean;
  accentClass?: string;
}

function formatLufs(value: number | null | undefined): string {
  if (value == null || !Number.isFinite(value)) {
    return "—";
  }
  return `${value.toFixed(1)} LUFS`;
}

function formatGainDb(value: number): string {
  const sign = value > 0 ? "+" : "";
  return `${sign}${value.toFixed(1)} dB`;
}

function replayGainEquivalentDb(loudnessLufs: number): string {
  return formatGainDb(-18 - loudnessLufs);
}

function InfoRow({ label, value }: { label: string; value: string }): ReactElement {
  return (
    <div className="flex justify-between gap-3">
      <span className="text-zinc-500">{label}</span>
      <span className="font-medium tabular-nums text-zinc-200">{value}</span>
    </div>
  );
}

export function DeckInfoPopover({
  deck,
  disabled = false,
  accentClass = "text-zinc-400",
}: DeckInfoPopoverProps): ReactElement {
  const hasLoudness = deck.loudness_lufs != null && Number.isFinite(deck.loudness_lufs);
  const gainTrimDb = normToStripDb(deck.gain_trim);
  const totalGainDb = deck.auto_gain_db + gainTrimDb;

  return (
    <Popover>
      <PopoverTrigger
        aria-label="Deck gain details"
        disabled={disabled}
        className={`${buttonIcon} border-white/10 bg-white/5 hover:bg-white/10 disabled:opacity-35 ${accentClass}`}
      >
        <Info className="size-3.5" />
      </PopoverTrigger>
      <PopoverPopup align="start" side="bottom" sideOffset={6} className="w-56">
        <PopoverTitle className="text-sm">Gain</PopoverTitle>
        <PopoverDescription className="mt-1.5 space-y-1 text-xs text-zinc-300">
          <InfoRow label="Loudness" value={formatLufs(deck.loudness_lufs)} />
          <InfoRow
            label="ReplayGain"
            value={hasLoudness ? replayGainEquivalentDb(deck.loudness_lufs as number) : "—"}
          />
          <InfoRow label="Auto gain" value={formatGainDb(deck.auto_gain_db)} />
          <InfoRow label="Gain trim" value={formatGainDb(gainTrimDb)} />
          <InfoRow label="Total gain" value={formatGainDb(totalGainDb)} />
        </PopoverDescription>
      </PopoverPopup>
    </Popover>
  );
}
