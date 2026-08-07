import { Slider } from "@/components/ui/slider";
import { DeckButton } from "@/components/ui/deck-button";
import type { DeckAccent } from "@/lib/ui";
import { DECK_ACCENTS } from "@/lib/ui";
import {
  effectiveBpm,
  formatBpm,
  formatPitchPercent,
  formatTempoRange,
  nextTempoRange,
  pitchSliderToSpeed,
  speedToPitchSlider,
} from "@/lib/format";
import type { DeckStatus } from "@/types";

interface DeckTempoPanelProps {
  accent: DeckAccent;
  deck: DeckStatus;
  disabled?: boolean;
  onSpeedChange: (speed: number) => void;
  onTempoRangeChange: (tempoRange: number) => void;
  onToggleSync: (beatSync: boolean) => void;
  onSetMaster: () => void;
}

export function DeckTempoPanel({
  accent,
  deck,
  disabled,
  onSpeedChange,
  onTempoRangeChange,
  onToggleSync,
  onSetMaster,
}: DeckTempoPanelProps) {
  const accentStyles = DECK_ACCENTS[accent];
  const liveBpm = effectiveBpm(deck.bpm, deck.speed, deck.tempo_range);
  const sliderValue = speedToPitchSlider(deck.speed);
  const syncActive = deck.sync_mode !== "off";
  const beatSynced = deck.sync_mode === "beat";

  return (
    <div className="flex h-full min-h-0 w-18 shrink-0 flex-col overflow-hidden rounded-md border border-white/10 bg-zinc-950/80 shadow-inner sm:w-20">
      <div className="flex shrink-0 flex-col items-center gap-0.5 border-b border-white/8 px-1.5 py-1.5">
        <span
          className={`text-base font-bold leading-none tabular-nums tracking-tight sm:text-lg ${accentStyles.text}`}
        >
          {formatBpm(liveBpm)}
        </span>
        <span className="text-[10px] font-medium tabular-nums text-zinc-500">
          {formatPitchPercent(deck.speed, deck.tempo_range)}
        </span>
        <DeckButton
          type="button"
          size="toggle"
          disabled={disabled}
          title="Cycle tempo range"
          className="w-full tabular-nums tracking-normal normal-case"
          onClick={() => onTempoRangeChange(nextTempoRange(deck.tempo_range))}
        >
          {formatTempoRange(deck.tempo_range)}
        </DeckButton>
        <DeckButton
          type="button"
          size="sync"
          active={syncActive}
          disabled={disabled || deck.is_master}
          title={
            deck.is_master
              ? "Master deck — shift+click to set master"
              : beatSynced
                ? "Beat sync on — click to disable"
                : syncActive
                  ? "Tempo sync on — click to disable, shift+click for beat sync"
                  : "Tempo sync — shift+click for beat sync"
          }
          onClick={(event) => {
            onToggleSync(event.shiftKey);
          }}
        >
          {deck.is_master ? "M" : beatSynced ? "B" : syncActive ? "S" : "Sync"}
        </DeckButton>
        {!deck.is_master ? (
          <button
            type="button"
            className="text-[9px] font-medium uppercase tracking-wide text-zinc-500 hover:text-zinc-300"
            disabled={disabled}
            onClick={onSetMaster}
          >
            Set master
          </button>
        ) : (
          <span className="text-[9px] font-semibold uppercase tracking-wide text-emerald-400/90">
            Master
          </span>
        )}
      </div>

      <div className="flex min-h-0 flex-1 items-center justify-center px-2 py-2 [&_[data-slot=slider-control]]:h-full [&_[data-slot=slider-control]]:min-h-0 [&_[data-slot=slider-control]]:items-center">
        <Slider
          orientation="vertical"
          thumbAlignment="center"
          showIndicator={false}
          showMarkers
          centerNotch
          thumbVariant="fader"
          channelAccent={accent}
          min={0}
          max={100}
          step={0.05}
          value={sliderValue}
          disabled={disabled || syncActive}
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
