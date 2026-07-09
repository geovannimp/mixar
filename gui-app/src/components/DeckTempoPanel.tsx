import { Slider } from "@/components/ui/slider";
import type { DeckAccent } from "../lib/ui";
import { DECK_ACCENTS } from "../lib/ui";
import {
  effectiveBpm,
  formatBpm,
  formatPitchOffset,
  pitchSliderToSpeed,
  speedToPitchSlider,
} from "../lib/format";
import type { DeckStatus } from "../types";

interface DeckTempoPanelProps {
  accent: DeckAccent;
  deck: DeckStatus;
  disabled?: boolean;
  onSpeedChange: (speed: number) => void;
}

export function DeckTempoPanel({
  accent,
  deck,
  disabled,
  onSpeedChange,
}: DeckTempoPanelProps) {
  const accentStyles = DECK_ACCENTS[accent];
  const liveBpm = effectiveBpm(deck.bpm, deck.speed);
  const sliderValue = speedToPitchSlider(deck.speed);

  return (
    <div className="flex h-full min-h-0 w-[4.5rem] shrink-0 flex-col overflow-hidden rounded-md border border-white/10 bg-zinc-950/80 shadow-inner sm:w-[5rem]">
      <div className="flex shrink-0 flex-col items-center gap-0.5 border-b border-white/8 px-1.5 py-1.5">
        <span
          className={`text-base font-bold leading-none tabular-nums tracking-tight sm:text-lg ${accentStyles.text}`}
        >
          {formatBpm(liveBpm)}
        </span>
        <span className="text-[10px] font-medium tabular-nums text-zinc-500">
          {formatPitchOffset(deck.speed)}
        </span>
        <button
          type="button"
          className="mt-0.5 w-full rounded border border-white/12 bg-zinc-900/90 px-1 py-1 text-[9px] font-bold uppercase tracking-[0.2em] text-zinc-300 transition hover:bg-zinc-800 disabled:cursor-not-allowed disabled:opacity-45"
          disabled={disabled}
          title="Coming in Phase 2"
        >
          Sync
        </button>
      </div>

      <div className="flex min-h-0 flex-1 items-center justify-center px-2 py-2 [&_[data-slot=slider-control]]:h-full [&_[data-slot=slider-control]]:min-h-0 [&_[data-slot=slider-control]]:items-center">
        <Slider
          orientation="vertical"
          thumbAlignment="center"
          showIndicator={false}
          thumbVariant="fader"
          channelAccent={accent}
          min={0}
          max={100}
          value={sliderValue}
          disabled={disabled}
          aria-label="Tempo"
          className="h-full w-8"
          onValueChange={(value) => {
            const next = Array.isArray(value) ? (value[0] ?? 50) : value;
            onSpeedChange(pitchSliderToSpeed(next));
          }}
        />
      </div>
    </div>
  );
}
